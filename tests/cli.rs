use std::{
    io::Write,
    process::{Command, Stdio},
};
fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_envhole"))
}

#[test]
fn help_describes_commands() {
    let out = bin().arg("--help").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("send") && text.contains("receive"));
}

#[test]
fn stdin_is_validated_before_network_and_values_are_hidden() {
    let mut child = bin()
        .args(["send", "-", "--yes"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"1BAD=unique-secret-value")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(!out.status.success());
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("invalid assignment"));
    assert!(!text.contains("unique-secret-value"));
}

#[test]
fn decline_sender_before_network() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input");
    std::fs::write(&path, b"A=unique-secret-value\n").unwrap();
    let out = bin()
        .arg("send")
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("A=***") && text.contains("cancelled"));
    assert!(!text.contains("unique-secret-value"));
}

#[test]
fn receive_requires_output_and_refuses_existing_before_network() {
    assert!(
        !bin()
            .args(["receive", "1-test-code"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let file = tempfile::NamedTempFile::new().unwrap();
    let out = bin()
        .args(["receive", "1-test-code", "--output"])
        .arg(file.path())
        .arg("--yes")
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--force"));
}
