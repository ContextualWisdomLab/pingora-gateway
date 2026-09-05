/// Reports whether shell text contains an executable POSIX-style `$(` introducer.
///
/// Matching the closing parenthesis requires full shell grammar (`case` patterns, here-documents,
/// nested command substitutions, and subshells all affect it). This guard deliberately avoids a
/// partial matcher: once an active `$(` exists, authority analysis is scoped to the already-bounded
/// workflow step or Docker `RUN` command supplied by the caller.
fn contains_active_command_substitution(shell: &str) -> bool {
    let bytes = shell.as_bytes();
    let mut index = 0;
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if byte == b'\\' && !single_quoted {
            escaped = true;
            index += 1;
            continue;
        }
        if byte == b'\'' && !double_quoted {
            single_quoted = !single_quoted;
            index += 1;
            continue;
        }
        if byte == b'"' && !single_quoted {
            double_quoted = !double_quoted;
            index += 1;
            continue;
        }
        if !single_quoted && byte == b'$' && bytes.get(index + 1) == Some(&b'(') {
            return true;
        }
        index += 1;
    }

    false
}

/// Conservatively normalizes shell punctuation, quoting, and escapes into security-relevant words.
fn security_tokens(shell: &str) -> Vec<String> {
    let mut normalized = String::with_capacity(shell.len());
    let mut escaped = false;

    for character in shell.chars() {
        if escaped {
            normalized.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        match character {
            '\'' | '"' => {}
            '$' | '(' | ')' | '{' | '}' | ';' | '|' | '&' | '\n' | '\r' | '\t' => {
                normalized.push(' ');
            }
            _ => normalized.push(character),
        }
    }
    if escaped {
        normalized.push('\\');
    }

    normalized.split_whitespace().map(str::to_owned).collect()
}

/// Returns the executable basename for an absolute or relative command token.
fn command_basename(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

/// Splits a simple or Bash `+=` assignment into its variable name and assigned value.
fn assignment_parts(word: &str) -> Option<(&str, &str)> {
    let (name, value) = word.split_once('=')?;
    Some((name.strip_suffix('+').unwrap_or(name), value))
}

/// Returns the environment-variable name for a simple or Bash `+=` assignment.
fn assignment_name(word: &str) -> Option<&str> {
    assignment_parts(word).map(|(name, _)| name)
}

/// Collects unquoted `NAME=$(` assignments whose closure is intentionally not partially parsed.
///
/// The bounded shell parser cannot safely decide whether arbitrary unquoted command-substitution
/// syntax is assignment-only or command-local without implementing the full shell grammar. Such a
/// name is therefore treated as ambiguous executable authority only if a later command word expands
/// that same parameter. Quoted command-local assignments remain handled by normal segment parsing.
fn unquoted_command_substitution_assignment_names(shell: &str) -> Vec<String> {
    let bytes = shell.as_bytes();
    let mut names = Vec::new();
    let mut index = 0;
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if byte == b'\\' && !single_quoted {
            escaped = true;
            index += 1;
            continue;
        }
        if byte == b'\'' && !double_quoted {
            single_quoted = !single_quoted;
            index += 1;
            continue;
        }
        if byte == b'"' && !single_quoted {
            double_quoted = !double_quoted;
            index += 1;
            continue;
        }

        if !single_quoted
            && !double_quoted
            && byte == b'='
            && bytes.get(index + 1) == Some(&b'$')
            && bytes.get(index + 2) == Some(&b'(')
        {
            let mut name_end = index;
            if name_end > 0 && bytes[name_end - 1] == b'+' {
                name_end -= 1;
            }
            let mut name_start = name_end;
            while name_start > 0
                && (bytes[name_start - 1].is_ascii_alphanumeric() || bytes[name_start - 1] == b'_')
            {
                name_start -= 1;
            }

            let has_boundary = name_start == 0
                || bytes[name_start - 1].is_ascii_whitespace()
                || matches!(bytes[name_start - 1], b';' | b'|' | b'&' | b'(' | b')');
            let name = &shell[name_start..name_end];
            let valid_name = !name.is_empty()
                && name
                    .as_bytes()
                    .first()
                    .is_some_and(|first| first.is_ascii_alphabetic() || *first == b'_')
                && name
                    .bytes()
                    .all(|candidate| candidate.is_ascii_alphanumeric() || candidate == b'_');

            if has_boundary && valid_name && !names.iter().any(|known| known == name) {
                names.push(name.to_owned());
            }
        }
        index += 1;
    }

    names
}

/// Splits bounded shell text into command-position words while preserving `$NAME` command words.
fn shell_command_segments(shell: &str) -> Vec<Vec<String>> {
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
            if matches!(character, ';' | '|' | '&' | '(' | ')') {
                flush_word(&mut word, &mut segment);
                flush_segment(&mut segment, &mut segments);
                continue;
            }
        }
        word.push(character);
    }

    flush_word(&mut word, &mut segment);
    flush_segment(&mut segment, &mut segments);
    segments
}

/// Returns a simple shell parameter name when the complete command word is `$NAME` or `${NAME...}`.
fn parameter_command_name(word: &str) -> Option<&str> {
    if let Some(name) = word.strip_prefix('$') {
        if !name.starts_with('{')
            && !name.is_empty()
            && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Some(name);
        }
    }

    let expression = word.strip_prefix("${")?;
    let closing = expression.rfind('}')?;
    if closing + 1 != expression.len() {
        return None;
    }
    let body = &expression[..closing];
    let name_end = body
        .bytes()
        .position(|byte| !(byte.is_ascii_alphanumeric() || byte == b'_'))
        .unwrap_or(body.len());
    (name_end > 0).then_some(&body[..name_end])
}

/// Applies one persistent shell assignment to the recorded Cargo command aliases.
fn apply_persistent_cargo_assignment(aliases: &mut Vec<String>, name: &str, value: &str) {
    aliases.retain(|alias| alias != name);
    if command_basename(value) == "cargo" || contains_active_command_substitution(value) {
        aliases.push(name.to_owned());
    }
}

/// Rejects Cargo executable indirection through persistent shell variables in command position.
fn contains_variable_cargo_command(shell: &str) -> bool {
    let ambiguous_dynamic_aliases = unquoted_command_substitution_assignment_names(shell);
    let mut aliases: Vec<String> = Vec::new();

    for segment in shell_command_segments(shell) {
        let mut index = 0;
        let mut assignment_prefix = Vec::new();
        while let Some(word) = segment.get(index) {
            let Some((name, value)) = assignment_parts(word) else {
                break;
            };
            assignment_prefix.push((name, value));
            index += 1;
        }

        let Some(command) = segment.get(index) else {
            for (name, value) in assignment_prefix {
                apply_persistent_cargo_assignment(&mut aliases, name, value);
            }
            continue;
        };
        let command_basename = command_basename(command);

        if matches!(command_basename, "export" | "readonly") {
            // POSIX `export` and `readonly` are special builtins, so assignment prefixes and
            // assignment operands affect the current shell rather than only a child command.
            for (name, value) in assignment_prefix {
                apply_persistent_cargo_assignment(&mut aliases, name, value);
            }
            for word in &segment[index + 1..] {
                let Some((name, value)) = assignment_parts(word) else {
                    continue;
                };
                apply_persistent_cargo_assignment(&mut aliases, name, value);
            }
            continue;
        }

        if command_basename == "unset" {
            for name in &segment[index + 1..] {
                aliases.retain(|alias| alias != name);
            }
            continue;
        }

        // Assignment prefixes on an ordinary command are command-local environment entries. They
        // must not be remembered as parent-shell aliases after that command returns.
        if parameter_command_name(command).is_some_and(|name| {
            aliases.iter().any(|alias| alias == name)
                || ambiguous_dynamic_aliases.iter().any(|alias| alias == name)
        }) {
            return true;
        }
    }

    false
}

/// Collects shell variables that are explicitly assigned a Cargo executable path.
fn cargo_command_aliases(tokens: &[String]) -> Vec<String> {
    tokens
        .iter()
        .filter_map(|token| {
            let (name, value) = assignment_parts(token)?;
            (command_basename(value) == "cargo").then(|| name.to_owned())
        })
        .collect()
}

/// Reports whether a shell parameter expansion actively references a recorded Cargo command alias.
///
/// This scanner only identifies the parameter name at an unescaped `${...}` introducer outside
/// single quotes. It does not parse the expansion word or closing brace, so forms such as
/// `${CARGO:?}`, `${CARGO:-cargo}`, and `${CARGO=cargo}` share one fail-closed identity rule without
/// confusing the simple assignment token `CARGO=cargo` with a command invocation.
fn contains_active_cargo_parameter_expansion(shell: &str, aliases: &[String]) -> bool {
    let bytes = shell.as_bytes();
    let mut index = 0;
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if byte == b'\\' && !single_quoted {
            escaped = true;
            index += 1;
            continue;
        }
        if byte == b'\'' && !double_quoted {
            single_quoted = !single_quoted;
            index += 1;
            continue;
        }
        if byte == b'"' && !single_quoted {
            double_quoted = !double_quoted;
            index += 1;
            continue;
        }
        if !single_quoted && byte == b'$' && bytes.get(index + 1) == Some(&b'{') {
            let name_start = index + 2;
            let mut name_end = name_start;
            while bytes.get(name_end).is_some_and(|candidate| {
                candidate.is_ascii_alphanumeric() || *candidate == b'_'
            }) {
                name_end += 1;
            }
            if name_end > name_start {
                let name = &shell[name_start..name_end];
                if aliases.iter().any(|alias| alias == name) {
                    return true;
                }
            }
        }
        index += 1;
    }

    false
}

/// Reports whether a normalized command token resolves directly or through a local alias to Cargo.
fn is_cargo_command(token: &str, aliases: &[String]) -> bool {
    command_basename(token) == "cargo" || aliases.iter().any(|alias| token == alias)
}

/// Detects compiler authority within one already-bounded executable shell step or Docker `RUN`.
fn shell_changes_compiler_authority(shell: &str) -> bool {
    if contains_variable_cargo_command(shell) {
        return true;
    }
    if !contains_active_command_substitution(shell) {
        return false;
    }

    let tokens = security_tokens(shell);
    let cargo_aliases = cargo_command_aliases(&tokens);
    let contains_cargo = tokens
        .iter()
        .any(|token| is_cargo_command(token, &cargo_aliases))
        || contains_active_cargo_parameter_expansion(shell, &cargo_aliases);

    if contains_cargo
        && tokens.iter().any(|token| {
            assignment_name(token).is_some_and(|name| {
                matches!(name, "RUSTC" | "CARGO_BUILD_RUSTC" | "RUSTUP_TOOLCHAIN")
            })
        })
    {
        return true;
    }

    if tokens.windows(2).any(|window| {
        is_cargo_command(&window[0], &cargo_aliases)
            && window[1].starts_with('+')
            && window[1].len() > 1
    }) {
        return true;
    }

    tokens.iter().enumerate().any(|(index, token)| {
        if command_basename(token) != "rustup" {
            return false;
        }
        matches!(
            tokens.get(index + 1).map(String::as_str),
            Some("default" | "override" | "run")
        ) || (tokens.get(index + 1).map(String::as_str) == Some("toolchain")
            && tokens.get(index + 2).map(String::as_str) == Some("install"))
    })
}

/// Fails closed when shell indirection can hide alternate Cargo compiler authority.
pub fn assert_no_hidden_compiler_authority(context: &str, shell: &str) {
    assert!(
        !shell_changes_compiler_authority(shell),
        "{context} must not hide alternate Cargo compiler authority behind shell indirection"
    );
}
