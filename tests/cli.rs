use std::{
    io::Write,
    net::TcpListener,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
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
    assert!(text.contains("--rendezvous-url") && text.contains("--transit-relay"));
    assert!(text.contains("--timeout-seconds") && text.contains("default: 600"));
}

#[test]
fn network_timeout_is_bounded() {
    for invalid in ["0", "86401"] {
        let out = bin()
            .args([
                "--timeout-seconds",
                invalid,
                "send",
                "/path/that/does/not/exist",
                "--yes",
            ])
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("1..=86400"));
    }
}

#[test]
fn network_timeout_stops_a_stalled_rendezvous() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (_connection, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_secs(2));
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.env");
    std::fs::write(&path, b"A=secret\n").unwrap();

    let started = Instant::now();
    let out = bin()
        .args([
            "--rendezvous-url",
            &format!("ws://{address}/v1"),
            "--timeout-seconds",
            "1",
            "send",
        ])
        .arg(path)
        .arg("--yes")
        .output()
        .unwrap();
    let elapsed = started.elapsed();
    server.join().unwrap();

    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("network transfer timed out"));
    assert!(elapsed < Duration::from_millis(1900));
}

#[test]
fn send_defaults_to_four_code_words_and_bounds_overrides() {
    let help = bin().args(["send", "--help"]).output().unwrap();
    assert!(help.status.success());
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(text.contains("--code-words"));
    assert!(text.contains("default: 4"));

    for invalid in ["1", "7"] {
        let out = bin()
            .args([
                "send",
                "/path/that/does/not/exist",
                "--code-words",
                invalid,
                "--yes",
            ])
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("2..=6"));
    }
}

#[test]
fn invalid_server_urls_fail_before_reading_payload() {
    let out = bin()
        .args([
            "--rendezvous-url",
            "not-a-websocket-url",
            "--transit-relay",
            "not-a-relay-url",
            "send",
            "/path/that/does/not/exist",
            "--yes",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("invalid rendezvous URL"));
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
    assert!(text.contains("A=••••••••") && text.contains("cancelled"));
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
