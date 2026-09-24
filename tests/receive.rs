use envhole::{BoundedBuffer, MAX_PAYLOAD};
use futures_lite::io::AsyncWriteExt;

#[test]
fn malicious_stream_cannot_exceed_limit() {
    futures_lite::future::block_on(async {
        let mut sink = BoundedBuffer::default();
        sink.write_all(&vec![b'x'; MAX_PAYLOAD]).await.unwrap();
        assert!(sink.write_all(b"x").await.is_err());
        assert_eq!(sink.into_bytes().len(), MAX_PAYLOAD);
    });
}

#[test]
fn sink_enforces_advertised_size_before_upstream_accounting() {
    futures_lite::future::block_on(async {
        let mut sink = BoundedBuffer::with_limit(3).unwrap();
        assert!(sink.write_all(b"four").await.is_err());
        assert!(sink.into_bytes().is_empty());
        assert!(BoundedBuffer::with_limit(MAX_PAYLOAD + 1).is_err());
    });
}

#[test]
fn receive_requires_approval_and_validation_before_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out");
    let bytes = b"A='synthetic'\r\n";
    assert!(
        envhole::save_received(bytes, &path, false, |names| {
            assert_eq!(names, ["A"]);
            Ok(false)
        })
        .is_err()
    );
    assert!(!path.exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    assert!(
        envhole::save_received(b"BAD-NAME=value", &path, false, |_| {
            panic!("must validate before asking")
        })
        .is_err()
    );
    envhole::save_received(bytes, &path, false, |_| Ok(true)).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}
