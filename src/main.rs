use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

mod linter;

use linter::{lint_text, Mode, Severity};

fn main() -> ExitCode {
    let mut lenient = false;
    let mut paths: Vec<String> = Vec::new();

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--lenient" => lenient = true,
            "--help" | "-h" => {
                print_usage();
                return ExitCode::from(0);
            }
            other if other.starts_with('-') && other != "-" => {
                eprintln!("dice-lint: unknown flag '{other}'");
                print_usage();
                return ExitCode::from(2);
            }
            other => paths.push(other.to_string()),
        }
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
            println!("{path}:{}:{}: {}: {} [{}]", f.line, f.col, f.severity.as_str(), f.message, f.rule);
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
    eprintln!("usage: dice-lint [--lenient] <file>...");
    eprintln!("       dice-lint [--lenient] -   (read from stdin)");
}
