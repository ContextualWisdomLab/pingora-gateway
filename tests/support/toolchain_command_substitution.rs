/// Finds executable POSIX-style command substitutions while respecting shell quoting and escapes.
fn command_substitutions(shell: &str) -> Vec<String> {
    let bytes = shell.as_bytes();
    let mut substitutions = Vec::new();
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
            let (end, body) = command_substitution_body(shell, index + 2);
            substitutions.push(body.to_owned());
            index = end + 1;
            continue;
        }
        index += 1;
    }

    substitutions
}

/// Returns the matching `)` and body for one active `$(` command substitution.
fn command_substitution_body(shell: &str, body_start: usize) -> (usize, &str) {
    let bytes = shell.as_bytes();
    let mut depth = 1usize;
    let mut index = body_start;
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
        if single_quoted {
            index += 1;
            continue;
        }
        if byte == b'$' && bytes.get(index + 1) == Some(&b'(') {
            depth += 1;
            index += 2;
            continue;
        }
        if byte == b'(' {
            depth += 1;
            index += 1;
            continue;
        }
        if byte == b')' {
            depth -= 1;
            if depth == 0 {
                return (index, &shell[body_start..index]);
            }
        }
        index += 1;
    }

    panic!("active command substitution must have a matching closing parenthesis");
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
            '(' | ')' | '{' | '}' | ';' | '|' | '&' | '\n' | '\r' | '\t' => normalized.push(' '),
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

/// Returns the environment-variable name for a simple or Bash `+=` assignment.
fn assignment_name(word: &str) -> Option<&str> {
    let (name, _) = word.split_once('=')?;
    Some(name.strip_suffix('+').unwrap_or(name))
}

/// Detects compiler authority that can execute inside a command-substitution subshell.
fn substitution_changes_compiler_authority(body: &str) -> bool {
    let tokens = security_tokens(body);
    let contains_cargo = tokens
        .iter()
        .any(|token| command_basename(token) == "cargo");

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
        command_basename(&window[0]) == "cargo"
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

/// Fails closed when executable `$(...)` text can select an alternate compiler for a release path.
pub fn assert_no_hidden_compiler_authority(context: &str, shell: &str) {
    for substitution in command_substitutions(shell) {
        assert!(
            !substitution_changes_compiler_authority(&substitution),
            "{context} must not hide Cargo compiler authority inside command substitution: {substitution}"
        );
    }
}
