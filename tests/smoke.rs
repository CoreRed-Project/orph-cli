/// Smoke tests for orph CLI — exit-code and JSON validity checks.
use assert_cmd::Command;

#[test]
fn sys_status_exits_ok() {
    let mut cmd = Command::cargo_bin("orph").unwrap();
    cmd.args(["sys", "status"]);
    cmd.assert().success();
}

#[test]
fn pet_status_json_is_valid() {
    let mut cmd = Command::cargo_bin("orph").unwrap();
    cmd.args(["pet", "status", "--json"]);
    let output = cmd.output().expect("failed to run orph");
    // Must exit 0
    assert!(output.status.success(), "orph pet status --json failed");
    // Output must be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    assert!(!trimmed.is_empty(), "empty JSON output");
    serde_json::from_str::<serde_json::Value>(trimmed)
        .expect("orph pet status --json did not produce valid JSON");
}
