/// Splits bounded workflow/Docker shell text into simple command segments while preserving
/// parameter expansions as command words. This is intentionally narrower than a shell parser:
/// release scripts that need more complex command-position grammar must extend this contract first.
fn command_segments(shell: &str) -> Vec<Vec<String>> {
    let normalized = shell.replace("\\\r\n", "").replace("\\\n", "");
    let mut segments = Vec::new();
    let mut segment = Vec::new();
    let mut word = String::new();
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;
    let mut comment = false;

    let flush_word = |word: &mut String, segment: &mut Vec<String>| {
        if !word.is_empty() {
            segment.push(std::mem::take(word));
        }
    };
    let flush_segment = |segment: &mut Vec<String>, segments: &mut Vec<Vec<String>>| {
        if !segment.is_empty() {
            segments.push(std::mem::take(segment));
        }
    };

    for character in normalized.chars() {
        if comment {
            if character == '\n' {
                comment = false;
                flush_word(&mut word, &mut segment);
                flush_segment(&mut segment, &mut segments);
            }
            continue;
        }
        if escaped {
            word.push(character);
            escaped = false;
            continue;
        }
        if !single_quoted && character == '\\' {
            escaped = true;
            continue;
        }
        if !double_quoted && character == '\'' {
            single_quoted = !single_quoted;
            continue;
        }
        if !single_quoted && character == '"' {
            double_quoted = !double_quoted;
            continue;
        }
        if !single_quoted && !double_quoted {
            if character == '#' && word.is_empty() {
                comment = true;
                continue;
            }
            if character.is_whitespace() {
                flush_word(&mut word, &mut segment);
                if character == '\n' {
                    flush_segment(&mut segment, &mut segments);
                }
                continue;
            }
            if matches!(character, ';' | '|' | '&') {
                flush_word(&mut word, &mut segment);
                flush_segment(&mut segment, &mut segments);
                continue;
            }
        }
        word.push(character);
    }

    assert!(
        !single_quoted && !double_quoted && !escaped,
        "compiler-authority shell contract requires balanced quoting/escaping"
    );
    flush_word(&mut word, &mut segment);
    flush_segment(&mut segment, &mut segments);
    segments
}

fn assignment_parts(word: &str) -> Option<(&str, &str)> {
    let (name, value) = word.split_once('=')?;
    Some((name.strip_suffix('+').unwrap_or(name), value))
}

fn command_basename(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

fn parameter_name(word: &str) -> Option<&str> {
    if let Some(name) = word.strip_prefix('$') {
        if !name.starts_with('{')
            && !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Some(name);
        }
    }

    let body = word.strip_prefix("${")?.strip_suffix('}')?;
    (!body.is_empty()
        && body
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'))
    .then_some(body)
}

fn set_alias(aliases: &mut Vec<String>, name: &str, value: &str) {
    aliases.retain(|alias| alias != name);
    if command_basename(value) == "cargo" {
        aliases.push(name.to_owned());
    }
}

/// Resolves a command position through the POSIX `command` builtin and GNU `env` utility.
/// Query-only `command -v/-V` and malformed/unknown wrapper options are non-executing for this
/// contract and therefore return `None` instead of guessing a child command boundary.
fn unwrap_command_position(segment: &[String], mut index: usize) -> Option<(&str, usize)> {
    loop {
        let command = command_basename(segment.get(index)?);
        index += 1;

        if command == "command" {
            loop {
                match segment.get(index).map(String::as_str) {
                    Some("-p") => index += 1,
                    Some("--") => {
                        index += 1;
                        break;
                    }
                    Some("-v" | "-V") => return None,
                    Some(option) if option.starts_with('-') => return None,
                    _ => break,
                }
            }
            continue;
        }

        if command == "env" {
            let mut options_active = true;
            loop {
                let argument = segment.get(index)?.as_str();
                if options_active {
                    if argument == "--" || argument == "-" {
                        options_active = false;
                        index += 1;
                        continue;
                    }

                    if let Some(long_option) = argument.strip_prefix("--") {
                        let (name, attached) = long_option
                            .split_once('=')
                            .map_or((long_option, false), |(name, _)| (name, true));
                        match name {
                            "null"
                            | "ignore-environment"
                            | "default-signal"
                            | "ignore-signal"
                            | "block-signal"
                            | "list-signal-handling"
                            | "debug" => {
                                index += 1;
                                continue;
                            }
                            "argv0" | "unset" | "chdir" => {
                                index += 1;
                                if !attached {
                                    segment.get(index)?;
                                    index += 1;
                                }
                                continue;
                            }
                            // `-S/--split-string` changes command tokenization and is governed by
                            // the separate shell-control contract rather than approximated here.
                            "split-string" | "help" | "version" => return None,
                            _ => return None,
                        }
                    }

                    if let Some(short_options) = argument.strip_prefix('-') {
                        let mut consumes_next = false;
                        for (offset, option) in short_options.char_indices() {
                            match option {
                                '0' | 'i' | 'v' => {}
                                'a' | 'u' | 'C' => {
                                    consumes_next =
                                        offset + option.len_utf8() == short_options.len();
                                    break;
                                }
                                'S' => return None,
                                _ => return None,
                            }
                        }
                        index += 1;
                        if consumes_next {
                            segment.get(index)?;
                            index += 1;
                        }
                        continue;
                    }

                    options_active = false;
                }

                if assignment_parts(argument).is_some() {
                    index += 1;
                    continue;
                }
                break;
            }
            continue;
        }

        return Some((segment.get(index - 1)?.as_str(), index));
    }
}

/// Reports explicit `cargo +toolchain` selection when Cargo is reached through a persistent shell
/// variable, including execution through `command` and GNU `env` wrappers.
pub fn has_wrapped_cargo_toolchain_selector(shell: &str) -> bool {
    let mut aliases = Vec::new();

    for segment in command_segments(shell) {
        let mut index = 0;
        let mut assignments = Vec::new();
        while let Some(word) = segment.get(index) {
            let Some((name, value)) = assignment_parts(word) else {
                break;
            };
            assignments.push((name, value));
            index += 1;
        }

        let Some(command) = segment.get(index) else {
            for (name, value) in assignments {
                set_alias(&mut aliases, name, value);
            }
            continue;
        };

        let command_name = command_basename(command);
        if matches!(command_name, "export" | "readonly" | "declare" | "typeset") {
            for (name, value) in assignments {
                set_alias(&mut aliases, name, value);
            }
            for word in &segment[index + 1..] {
                if let Some((name, value)) = assignment_parts(word) {
                    set_alias(&mut aliases, name, value);
                }
            }
            continue;
        }
        if command_name == "unset" {
            for name in &segment[index + 1..] {
                aliases.retain(|alias| alias != name);
            }
            continue;
        }

        let Some((resolved_command, argument_index)) = unwrap_command_position(&segment, index)
        else {
            continue;
        };
        let Some(alias) = parameter_name(resolved_command) else {
            continue;
        };
        if aliases.iter().any(|known| known == alias)
            && segment
                .get(argument_index)
                .is_some_and(|argument| argument.starts_with('+') && argument.len() > 1)
        {
            return true;
        }
    }

    false
}

pub fn assert_no_wrapped_cargo_toolchain_selector(context: &str, shell: &str) {
    assert!(
        !has_wrapped_cargo_toolchain_selector(shell),
        "{context} must not hide Cargo +toolchain selection behind a persistent executable alias"
    );
}
