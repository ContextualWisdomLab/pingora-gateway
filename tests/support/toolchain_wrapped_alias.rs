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

/// Reports explicit `cargo +toolchain` selection when Cargo is reached through a persistent shell
/// variable. Wrapper utilities are handled by the GREEN repair after the RED contract proves them.
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

        let Some(alias) = parameter_name(command) else {
            continue;
        };
        if aliases.iter().any(|known| known == alias)
            && segment
                .get(index + 1)
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
