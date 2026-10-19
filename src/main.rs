use std::env;
use std::fs;
use std::io::{self, BufRead, BufWriter, Read, Write};
use std::process::ExitCode;

use serde::{Deserialize, Serialize};
use utsc::{Diagnostic, Scanner, TYPESCRIPT_VERSION, Token};

const HELP: &str = "utsc — native Rust TypeScript compiler infrastructure\n\
Usage: utsc scan <file>...\n\
       utsc scan --stdin\n\
       utsc scan --jsonl\n\
       utsc --version\n\n\
scan emits one JSON record per source, including trivia and EOF.\n\
--jsonl accepts {\"name\":\"example.ts\",\"source\":\"let x = 1;\"} per line.\n\
Offsets are zero-based UTF-16 code units plus UTF-8 byte offsets.\n\
Exit codes: 0 scanned, 1 lexical diagnostics, 2 invocation/I/O/protocol error.\n\
This milestone does not parse, type-check, or emit JavaScript.\n";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    name: String,
    source: String,
}

#[derive(Serialize)]
struct Response<'a> {
    name: &'a str,
    tokens: Vec<Token<'a>>,
    diagnostics: &'a [Diagnostic],
}

fn scan(name: &str, source: &str, output: &mut impl Write) -> io::Result<bool> {
    let mut scanner = Scanner::new(source);
    let tokens: Vec<_> = scanner.by_ref().collect();
    let response = Response {
        name,
        tokens,
        diagnostics: scanner.diagnostics(),
    };
    serde_json::to_writer(&mut *output, &response)?;
    writeln!(output)?;
    Ok(!response.diagnostics.is_empty())
}

fn run() -> Result<u8, Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let Some(command) = args.next() else {
        return Err("missing command; use --help".into());
    };
    let stdout = io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    if command == "--help" || command == "-h" {
        if args.next().is_some() {
            return Err("--help accepts no arguments".into());
        }
        output.write_all(HELP.as_bytes())?;
        output.flush()?;
        return Ok(0);
    }
    if command == "--version" {
        if args.next().is_some() {
            return Err("--version accepts no arguments".into());
        }
        writeln!(
            output,
            "utsc {} (TypeScript target {TYPESCRIPT_VERSION}; scanner milestone)",
            env!("CARGO_PKG_VERSION")
        )?;
        output.flush()?;
        return Ok(0);
    }
    if command != "scan" {
        return Err("unknown command; use --help".into());
    }
    let paths: Vec<_> = args.collect();
    if paths.is_empty() {
        return Err("scan requires files, --stdin, or --jsonl".into());
    }
    let mut has_errors = false;
    if paths.len() == 1 && paths[0] == "--stdin" {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source)?;
        has_errors |= scan("<stdin>", &source, &mut output)?;
    } else if paths.len() == 1 && paths[0] == "--jsonl" {
        for (index, line) in io::stdin().lock().lines().enumerate() {
            let request: Request = serde_json::from_str(&line?)
                .map_err(|e| format!("JSONL line {}: {e}", index + 1))?;
            has_errors |= scan(&request.name, &request.source, &mut output)?;
        }
    } else {
        for path in paths {
            if path.to_str().is_some_and(|s| s.starts_with('-')) {
                return Err(
                    "unknown option or mixed input modes; prefix a filename with ./ if necessary"
                        .into(),
                );
            }
            let source = fs::read_to_string(&path)
                .map_err(|e| format!("{}: {e}", path.to_string_lossy()))?;
            has_errors |= scan(&path.to_string_lossy(), &source, &mut output)?;
        }
    }
    output.flush()?;
    Ok(u8::from(has_errors))
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            if error
                .downcast_ref::<io::Error>()
                .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe)
            {
                return ExitCode::SUCCESS;
            }
            eprintln!("utsc: {error}");
            ExitCode::from(2)
        }
    }
}
