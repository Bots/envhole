// Uses synthetic data only. Explicitly opt in to configured network infrastructure.
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "requires Magic Wormhole rendezvous/transit access"]
fn two_process_exact_byte_transfer() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("received");
    let payload = b"# smoke\r\nexport TOKEN='synthetic-only'\r\nEMPTY=\n";
    let mut sender = Process(
        Command::new(env!("CARGO_BIN_EXE_envhole"))
            .args(["send", "-", "--yes"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    sender.0.stdin.take().unwrap().write_all(payload).unwrap();
    let stdout = sender.0.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let line = line.unwrap();
            assert!(!line.contains("synthetic-only"));
            if let Some(code) = line.strip_prefix("Code: ") {
                tx.send(code.to_owned()).unwrap();
                return;
            }
        }
    });
    let code = rx
        .recv_timeout(Duration::from_secs(45))
        .expect("sender did not produce a code within 45 seconds");
    let mut receiver = Process(
        Command::new(env!("CARGO_BIN_EXE_envhole"))
            .args(["receive", &code, "--yes", "--output"])
            .arg(&output)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(90);
    loop {
        if let Some(status) = receiver.0.try_wait().unwrap() {
            if !status.success() {
                let mut stderr = String::new();
                receiver
                    .0
                    .stderr
                    .take()
                    .unwrap()
                    .read_to_string(&mut stderr)
                    .unwrap();
                panic!("receiver failed: {stderr}");
            }
            break;
        }
        assert!(std::time::Instant::now() < deadline, "receiver timed out");
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(std::fs::read(output).unwrap(), payload);
    let mut text = String::new();
    receiver
        .0
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    assert!(text.contains("2 variables") && text.contains("TOKEN=***"));
    assert!(!text.contains("synthetic-only"));
    loop {
        if let Some(status) = sender.0.try_wait().unwrap() {
            if !status.success() {
                let mut stderr = String::new();
                sender
                    .0
                    .stderr
                    .take()
                    .unwrap()
                    .read_to_string(&mut stderr)
                    .unwrap();
                panic!("sender failed: {stderr}");
            }
            break;
        }
        assert!(std::time::Instant::now() < deadline, "sender timed out");
        std::thread::sleep(Duration::from_millis(100));
    }
}
