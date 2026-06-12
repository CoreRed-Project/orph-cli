use assert_cmd::Command;
use rusqlite::Connection;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_script_execution_logs_to_history() {
    let home = TempDir::new().unwrap();
    let scripts_dir = home.path().join(".orph").join("scripts");
    fs::create_dir_all(&scripts_dir).unwrap();

    // Write a dummy script
    let script_path = scripts_dir.join("test_run");
    fs::write(
        &script_path,
        "#!/bin/sh\necho \"stdout hello\"\n>&2 echo \"stderr hello\"\n",
    )
    .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    // Execute script via orph CLI
    let mut cmd = Command::cargo_bin("orph").unwrap();
    cmd.env("HOME", home.path());
    cmd.args(["run", "test_run"]);
    cmd.assert().success();

    // Verify it exists in db
    let db_path = home.path().join(".orph").join("orph.db");
    let conn = Connection::open(&db_path).unwrap();

    let mut stmt = conn
        .prepare("SELECT script_name, exit_code, stdout, stderr FROM script_history")
        .unwrap();
    let mut rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .unwrap();

    let entry = rows.next().unwrap().unwrap();
    assert_eq!(entry.0, "test_run");
    assert_eq!(entry.1, 0);
    assert!(entry.2.contains("stdout hello"));
    assert!(entry.3.contains("stderr hello"));
}
