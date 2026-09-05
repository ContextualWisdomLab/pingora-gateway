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

/// Reports whether a normalized command token resolves directly or through a local alias to Cargo.
fn is_cargo_command(token: &str, aliases: &[String]) -> bool {
    command_basename(token) == "cargo" || aliases.iter().any(|alias| token == alias)
}

/// Detects compiler authority within one already-bounded executable shell step or Docker `RUN`.
fn shell_changes_compiler_authority(shell: &str) -> bool {
    if !contains_active_command_substitution(shell) {
        return false;
    }

    let tokens = security_tokens(shell);
    let cargo_aliases = cargo_command_aliases(&tokens);
    let contains_cargo = tokens
        .iter()
        .any(|token| is_cargo_command(token, &cargo_aliases));

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

/// Fails closed when an active `$()` shares a bounded shell step with alternate compiler authority.
pub fn assert_no_hidden_compiler_authority(context: &str, shell: &str) {
    assert!(
        !shell_changes_compiler_authority(shell),
        "{context} must not combine active command substitution with alternate Cargo compiler authority"
    );
}
