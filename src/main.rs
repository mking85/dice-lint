use std::env;
use std::fs;
use std::process::ExitCode;

mod linter;

use linter::{lint_text, Mode, Severity};

fn main() -> ExitCode {
    let mut lenient = false;
    let mut path: Option<String> = None;

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--lenient" => lenient = true,
            "--help" | "-h" => {
                print_usage();
                return ExitCode::from(0);
            }
            other if other.starts_with('-') => {
                eprintln!("dice-lint: unknown flag '{other}'");
                print_usage();
                return ExitCode::from(2);
            }
            other => {
                if path.is_some() {
                    eprintln!("dice-lint: only one file may be given at a time");
                    return ExitCode::from(2);
                }
                path = Some(other.to_string());
            }
        }
    }

    let Some(path) = path else {
        print_usage();
        return ExitCode::from(2);
    };

    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("dice-lint: could not read '{path}': {e}");
            return ExitCode::from(2);
        }
    };

    let mode = if lenient { Mode::Lenient } else { Mode::Strict };
    let findings = lint_text(&text, mode);

    let mut had_error = false;
    for f in &findings {
        if f.severity == Severity::Error {
            had_error = true;
        }
        println!("{path}:{}:{}: {}: {} [{}]", f.line, f.col, f.severity.as_str(), f.message, f.rule);
    }

    if had_error {
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    }
}

fn print_usage() {
    eprintln!("usage: dice-lint [--lenient] <file>");
}
