use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

mod linter;

use linter::{lint_text, Mode, Severity};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Text,
    Json,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut lenient = false;
    let mut format = Format::Text;
    let mut paths: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--lenient" => lenient = true,
            "--help" | "-h" => {
                print_usage();
                return ExitCode::from(0);
            }
            "--format" => {
                i += 1;
                let Some(value) = args.get(i) else {
                    eprintln!("dice-lint: --format requires a value ('text' or 'json')");
                    print_usage();
                    return ExitCode::from(2);
                };
                format = match value.as_str() {
                    "text" => Format::Text,
                    "json" => Format::Json,
                    other => {
                        eprintln!("dice-lint: unknown format '{other}', expected 'text' or 'json'");
                        print_usage();
                        return ExitCode::from(2);
                    }
                };
            }
            other if other.starts_with('-') && other != "-" => {
                eprintln!("dice-lint: unknown flag '{other}'");
                print_usage();
                return ExitCode::from(2);
            }
            other => paths.push(other.to_string()),
        }
        i += 1;
    }

    if paths.is_empty() {
        print_usage();
        return ExitCode::from(2);
    }

    let mode = if lenient { Mode::Lenient } else { Mode::Strict };
    let mut had_error = false;
    let mut had_read_error = false;

    for path in &paths {
        let text = if path == "-" {
            let mut buf = String::new();
            match io::stdin().read_to_string(&mut buf) {
                Ok(_) => buf,
                Err(e) => {
                    eprintln!("dice-lint: could not read stdin: {e}");
                    had_read_error = true;
                    continue;
                }
            }
        } else {
            match fs::read_to_string(path) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("dice-lint: could not read '{path}': {e}");
                    had_read_error = true;
                    continue;
                }
            }
        };

        for f in lint_text(&text, mode) {
            if f.severity == Severity::Error {
                had_error = true;
            }
            match format {
                Format::Text => {
                    println!("{path}:{}:{}: {}: {} [{}]", f.line, f.col, f.severity.as_str(), f.message, f.rule);
                }
                Format::Json => {
                    println!(
                        "{{\"path\":\"{}\",\"line\":{},\"col\":{},\"severity\":\"{}\",\"rule\":\"{}\",\"message\":\"{}\"}}",
                        json_escape(path),
                        f.line,
                        f.col,
                        f.severity.as_str(),
                        f.rule,
                        json_escape(&f.message),
                    );
                }
            }
        }
    }

    if had_read_error {
        ExitCode::from(2)
    } else if had_error {
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    }
}

fn print_usage() {
    eprintln!("usage: dice-lint [--lenient] [--format text|json] <file>...");
    eprintln!("       dice-lint [--lenient] [--format text|json] -   (read from stdin)");
}

/// Escapes a string for use inside a JSON string literal. Findings echo
/// back arbitrary bytes from the scanned text (in `message`) and from the
/// command line (in `path`), so this can't assume the input is already
/// well-formed JSON-safe text.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escape_plain_text_is_unchanged() {
        assert_eq!(json_escape("3d6 rolls fine"), "3d6 rolls fine");
    }

    #[test]
    fn json_escape_quotes_and_backslashes() {
        assert_eq!(json_escape(r#"say "hi""#), r#"say \"hi\""#);
        assert_eq!(json_escape(r"C:\dice"), r"C:\\dice");
    }

    #[test]
    fn json_escape_control_characters() {
        assert_eq!(json_escape("a\nb\tc\rd"), "a\\nb\\tc\\rd");
        assert_eq!(json_escape("\u{7}"), "\\u0007");
    }
}
