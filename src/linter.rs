//! Finds dice-notation tokens (`3d6`, `d20`, `4d6kh3`, ...) inside plain text
//! and checks each one against a fixed rule set.
//!
//! A "word" is anything between whitespace. We only attempt to parse a word
//! as dice notation if there is a 'd'/'D' whose prefix (from the start of the
//! word) is either empty or all digits -- that keeps ordinary English words
//! like "delayed" or "middle" from being misread as dice tokens, without
//! needing a regex engine.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Strict,
    Lenient,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

pub struct Finding {
    pub line: usize,
    pub col: usize,
    pub severity: Severity,
    pub rule: &'static str,
    pub message: String,
}

pub fn lint_text(text: &str, mode: Mode) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (i, line) in text.lines().enumerate() {
        lint_line(line, i + 1, mode, &mut findings);
    }
    findings
}

fn lint_line(line: &str, line_no: usize, mode: Mode, out: &mut Vec<Finding>) {
    for word in line.split_whitespace() {
        let byte_start = word.as_ptr() as usize - line.as_ptr() as usize;
        let col = line[..byte_start].chars().count() + 1;
        lint_word(word, line_no, col, mode, out);
    }
}

fn lint_word(raw: &str, line: usize, col: usize, mode: Mode, out: &mut Vec<Finding>) {
    if !raw.is_ascii() {
        return;
    }

    // Strip the light punctuation that shows up around dice terms in prose,
    // e.g. "(3d6)" or "1d20,".
    let mut word = raw;
    if let Some(rest) = word.strip_prefix('(') {
        word = rest;
    }
    word = word.trim_end_matches([')', ',', '.', ';', ':']);

    let Some(split) = find_split(word) else {
        return;
    };

    let count_str = &word[..split];
    let d_byte = word.as_bytes()[split];
    let rest = &word[split + 1..];
    let sides_str = leading_digits(rest);
    let modifiers_str = &rest[sides_str.len()..];

    let mut push = |severity: Severity, rule: &'static str, message: String| {
        out.push(Finding { line, col, severity, rule, message });
    };

    if sides_str.is_empty() {
        push(
            Severity::Error,
            "missing-sides",
            format!("'{word}' is missing the number of sides after 'd'"),
        );
        return;
    }

    let sides_val: Option<u64> = sides_str.parse().ok();
    let Some(sides_val) = sides_val else {
        push(Severity::Error, "sides-out-of-range", format!("'{word}' has a sides count too large to evaluate"));
        return;
    };

    let count_val: Option<u64> = if count_str.is_empty() {
        None
    } else {
        match count_str.parse() {
            Ok(v) => Some(v),
            Err(_) => {
                push(Severity::Error, "count-out-of-range", format!("'{word}' has a dice count too large to evaluate"));
                return;
            }
        }
    };

    if let Err(bad) = check_modifiers(modifiers_str) {
        push(Severity::Error, "bad-modifier", format!("'{word}' has an unrecognized modifier near '{bad}'"));
        return;
    }

    if count_val == Some(0) {
        push(Severity::Error, "zero-count", format!("'{word}' rolls zero dice"));
    }
    if sides_val == 0 {
        push(Severity::Error, "zero-sides", format!("'{word}' has a die with zero sides"));
    }

    if mode == Mode::Lenient {
        return;
    }

    if d_byte == b'D' {
        push(Severity::Error, "uppercase-d", format!("'{word}' uses uppercase 'D'; write it lowercase"));
    }
    if count_val.is_none() {
        push(Severity::Error, "implicit-count", format!("'{word}' omits the dice count; write '1{word}'"));
    }
    if count_str.len() > 1 && count_str.starts_with('0') {
        push(Severity::Warning, "leading-zero", format!("'{word}' has a leading zero in the dice count"));
    }
    if sides_str.len() > 1 && sides_str.starts_with('0') {
        push(Severity::Warning, "leading-zero", format!("'{word}' has a leading zero in the sides count"));
    }
    if sides_val == 1 {
        push(Severity::Warning, "flat-die", format!("'{word}' always rolls 1; a flat modifier is clearer"));
    }
    if let Some(c) = count_val {
        if c > 100 {
            push(Severity::Warning, "large-count", format!("'{word}' rolls an unusually large number of dice"));
        }
    }
    if sides_val > 1000 {
        push(Severity::Warning, "large-sides", format!("'{word}' uses an unusually large side count"));
    }
}

/// Finds the byte index of the 'd'/'D' that separates dice count from sides,
/// or `None` if the word does not look like an attempted dice term at all.
fn find_split(word: &str) -> Option<usize> {
    let bytes = word.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'd' && b != b'D' {
            continue;
        }
        let prefix = &word[..i];
        if !prefix.is_empty() && prefix.bytes().all(|c| c.is_ascii_digit()) {
            return Some(i);
        }
        if prefix.is_empty() {
            if let Some(&next) = bytes.get(i + 1) {
                if next.is_ascii_digit() {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn leading_digits(s: &str) -> &str {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    &s[..end]
}

/// Consumes a chain of dice modifiers (`kh1`, `dl2`, `r1`, `!`, `+3`, `-1`, ...).
/// Returns the leftover slice on the first token it cannot make sense of.
fn check_modifiers(mut s: &str) -> Result<(), &str> {
    while !s.is_empty() {
        if let Some(rest) = s.strip_prefix("kh").or_else(|| s.strip_prefix("kl")).or_else(|| s.strip_prefix("dh")).or_else(|| s.strip_prefix("dl")) {
            let digits = leading_digits(rest);
            if digits.is_empty() {
                return Err(s);
            }
            s = &rest[digits.len()..];
            continue;
        }
        if let Some(rest) = s.strip_prefix('r') {
            let digits = leading_digits(rest);
            if digits.is_empty() {
                return Err(s);
            }
            s = &rest[digits.len()..];
            continue;
        }
        if let Some(rest) = s.strip_prefix('!') {
            let digits = leading_digits(rest);
            s = &rest[digits.len()..];
            continue;
        }
        if let Some(rest) = s.strip_prefix('+').or_else(|| s.strip_prefix('-')) {
            let digits = leading_digits(rest);
            if digits.is_empty() {
                return Err(s);
            }
            s = &rest[digits.len()..];
            continue;
        }
        return Err(s);
    }
    Ok(())
}
