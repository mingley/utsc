use std::io::Write;
use std::process::{Command, Output, Stdio};

fn invoke(args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_utsc"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn stdin_produces_utf16_spans_and_one_eof() {
    let result = invoke(&["scan", "--stdin"], "let 变量 = '😀';".as_bytes());
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    let response: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(response["tokens"][2]["text"], "变量");
    assert_eq!(response["tokens"][2]["start"], 4);
    assert_eq!(response["tokens"][2]["end"], 6);
    assert_eq!(response["diagnostics"], serde_json::json!([]));
}

#[test]
fn lexical_errors_still_return_tokens_and_exit_one() {
    let result = invoke(&["scan", "--stdin"], b"/* bad");
    assert_eq!(result.status.code(), Some(1));
    let response: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(response["tokens"][0]["unterminated"], true);
    assert_eq!(response["diagnostics"][0]["code"], "unterminated_comment");
}

#[test]
fn jsonl_processes_multiple_requests_including_errors() {
    let result = invoke(
        &["scan", "--jsonl"],
        b"{\"name\":\"a.ts\",\"source\":\"/*\"}\n{\"name\":\"b.ts\",\"source\":\"const x = 1;\"}\n",
    );
    assert_eq!(result.status.code(), Some(1));
    let stdout = String::from_utf8(result.stdout).unwrap();
    let records: Vec<serde_json::Value> = stdout
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["name"], "a.ts");
    assert_eq!(records[1]["name"], "b.ts");
    assert_eq!(records[1]["diagnostics"], serde_json::json!([]));
}

#[test]
fn io_and_protocol_errors_have_distinct_exit_status() {
    for (args, input) in [
        (vec!["scan", "--jsonl"], &b"{bad}\n"[..]),
        (vec!["scan", "--stdin"], &b"\xff"[..]),
        (vec!["scan", "this-file-does-not-exist.ts"], &b""[..]),
        (vec!["check"], &b""[..]),
        (vec!["scan", "--stdin", "--jsonl"], &b""[..]),
    ] {
        let result = invoke(&args, input);
        assert_eq!(result.status.code(), Some(2));
        assert!(!result.stderr.is_empty());
    }
}

#[test]
fn documented_file_and_version_commands_work() {
    let file = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/representative.ts"
    );
    let result = invoke(&["scan", file], b"");
    assert!(result.status.success());
    let result = invoke(&["--version"], b"");
    assert!(result.status.success());
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains(utsc::TYPESCRIPT_VERSION)
    );
}
