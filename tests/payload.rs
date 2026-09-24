use envhole::{MAX_PAYLOAD, manifest, preview, read_confirmation, read_payload};

#[test]
fn metadata_preserves_input_and_masks_values() {
    let original = b"# comment\r\n\r\nexport API_KEY=\"top-secret\"\r\nEMPTY=\nSINGLE='$(touch nope)'\nRAW=${NO_EVAL}\n";
    let bytes = read_payload(&original[..]).unwrap();
    assert_eq!(bytes, original);
    let names = manifest(&bytes).unwrap();
    assert_eq!(names, ["API_KEY", "EMPTY", "SINGLE", "RAW"]);
    assert_eq!(
        preview(&names),
        "4 variables\nAPI_KEY=***\nEMPTY=***\nSINGLE=***\nRAW=***\n"
    );
}

#[test]
fn malformed_and_duplicate_keys_are_rejected_without_echoing_input() {
    for bytes in [
        b"SECRET=value\nSECRET=other".as_slice(),
        b"1BAD=secret",
        b"BAD-NAME=secret",
        b"secret",
        b"=secret",
        b"A=\"secret",
        b"A='secret' garbage",
        b"A=secret\0",
        b"SAFE=x\rLD_PRELOAD=hidden",
        "SAFE=x\u{2028}LD_PRELOAD=hidden".as_bytes(),
        "SAFE=x\u{2029}LD_PRELOAD=hidden".as_bytes(),
        b"A=\xff",
    ] {
        let error = manifest(bytes).unwrap_err().to_string();
        assert!(!error.contains("secret"), "{error}");
    }
}

#[test]
fn confirmation_input_is_strict_and_bounded() {
    assert!(read_confirmation(&b"y\n"[..]).unwrap());
    assert!(read_confirmation(&b"YES\r\n"[..]).unwrap());
    assert!(!read_confirmation(&b"no\n"[..]).unwrap());
    assert!(!read_confirmation(&b""[..]).unwrap());
    assert!(read_confirmation(&b"yes-then-unbounded-data"[..]).is_err());
}

#[test]
fn size_is_bounded_including_streams() {
    assert_eq!(
        read_payload(&vec![b' '; MAX_PAYLOAD][..]).unwrap().len(),
        MAX_PAYLOAD
    );
    assert!(
        read_payload(&vec![b' '; MAX_PAYLOAD + 1][..])
            .unwrap_err()
            .to_string()
            .contains("1 MiB")
    );
}

#[test]
fn quoted_multiline_comments_and_escapes() {
    assert_eq!(
        manifest(b"  export A = \"one\\\"two\nthree\" # comment\nB='a\nb'\n_C=x # tail\n").unwrap(),
        ["A", "B", "_C"]
    );
}
