use envhole::write_atomic;
use std::fs;

#[test]
fn exact_bytes_and_overwrite_policy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".env");
    let bytes = b"# original\r\nA='value'\r\n\n";
    write_atomic(&path, bytes, false).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(write_atomic(&path, b"A=new", false).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    write_atomic(&path, b"A=new", true).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"A=new");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn private_permissions_and_symlink_refusal() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target");
    fs::write(&target, b"old").unwrap();
    let link = dir.path().join("link");
    symlink(&target, &link).unwrap();
    for force in [false, true] {
        assert!(write_atomic(&link, b"A=new", force).is_err());
    }
    assert_eq!(fs::read(&target).unwrap(), b"old");
    write_atomic(&target, b"A=new", true).unwrap();
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let dangling = dir.path().join("dangling");
    symlink(dir.path().join("absent"), &dangling).unwrap();
    assert!(write_atomic(&dangling, b"A=new", true).is_err());
}
