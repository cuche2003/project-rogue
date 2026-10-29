use std::{
    io::Write,
    process::{Command, Stdio},
};

use serde_json::Value;

fn run_agent(input: &str) -> (std::process::Output, Vec<Value>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_terminal-ui"))
        .args(["--agent"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let responses = String::from_utf8(output.stdout.clone())
        .unwrap()
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<Vec<Value>, _>>()
        .unwrap();
    (output, responses)
}

fn game_response(value: &Value) -> &Value {
    assert!(value.get("result").is_some(), "{value}");
    assert!(value.get("events").is_some(), "{value}");
    assert!(value.get("snapshot").is_some(), "{value}");
    value
}

#[test]
fn agent_returns_json_state_for_commands_and_quits() {
    let (output, responses) = run_agent("wait\nquit\n");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(responses.len(), 2);
    let response = game_response(&responses[0]);
    assert_eq!(response["result"], "Applied");
    assert_eq!(response["snapshot"]["phase"], "Playing");
    assert_eq!(responses[1], serde_json::json!({"ok": true, "quit": true}));
    assert!(output.stderr.is_empty());
}

#[test]
fn agent_reports_parse_errors_without_stopping() {
    let (output, responses) = run_agent("move\nuse 9\nwhat\nquit\n");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(responses.len(), 4);
    assert_eq!(responses[0]["error"], "move requires a direction");
    assert_eq!(
        responses[1]["error"],
        "use slot must be an integer from 0 to 8"
    );
    assert_eq!(responses[2]["error"], "unknown command: what");
    assert_eq!(responses[3], serde_json::json!({"ok": true, "quit": true}));
}

#[test]
fn agent_supports_all_game_commands_with_json_responses() {
    let (output, responses) =
        run_agent("move right\nwait\npickup\nuse 0\ndescend\nrestart\nquit\n");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(responses.len(), 7);
    for response in &responses[..6] {
        game_response(response);
    }
    assert_eq!(responses[6], serde_json::json!({"ok": true, "quit": true}));
}

#[test]
fn agent_ignores_blank_lines() {
    let (output, responses) = run_agent("\n  \nwait\n\nquit\n");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(responses.len(), 2);
    game_response(&responses[0]);
    assert_eq!(responses[1], serde_json::json!({"ok": true, "quit": true}));
}
