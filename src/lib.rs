#![forbid(unsafe_code)]
#![deny(warnings)]

use anyhow::{Result, bail};
use std::{collections::HashSet, io::Read};

pub const MAX_PAYLOAD: usize = 1024 * 1024;

pub fn read_payload(reader: impl Read) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_PAYLOAD + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| anyhow::anyhow!("could not read payload"))?;
    check_size(bytes.len())?;
    Ok(bytes)
}

/// Read one small confirmation response without permitting an unbounded line.
pub fn read_confirmation(mut reader: impl Read) -> Result<bool> {
    let mut response = Vec::with_capacity(4);
    loop {
        let mut byte = [0u8; 1];
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) if byte[0] == b'\n' => break,
            Ok(_) if response.len() == 4 => bail!("confirmation response is too long"),
            Ok(_) => response.push(byte[0]),
            Err(_) => bail!("could not read confirmation"),
        }
    }
    let response = std::str::from_utf8(&response)
        .map_err(|_| anyhow::anyhow!("invalid confirmation response"))?;
    Ok(matches!(
        response.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn check_size(size: usize) -> Result<()> {
    if size > MAX_PAYLOAD {
        bail!("payload exceeds 1 MiB limit");
    }
    Ok(())
}

/// Parse metadata only. No expansion, evaluation, or value reconstruction.
pub fn manifest(bytes: &[u8]) -> Result<Vec<String>> {
    check_size(bytes.len())?;
    if bytes
        .iter()
        .enumerate()
        .any(|(index, byte)| *byte == b'\r' && bytes.get(index + 1) != Some(&b'\n'))
    {
        bail!("payload contains an unsupported line separator");
    }
    let text = std::str::from_utf8(bytes).map_err(|_| anyhow::anyhow!("payload must be UTF-8"))?;
    if text.chars().any(|c| {
        (c.is_control() && !matches!(c, '\n' | '\r' | '\t')) || matches!(c, '\u{2028}' | '\u{2029}')
    }) {
        bail!("payload contains forbidden control characters");
    }
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    let mut lines = text.lines().enumerate();
    while let Some((line, raw)) = lines.next() {
        let invalid = || anyhow::anyhow!("invalid assignment at line {}", line + 1);
        let mut s = raw.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        if let Some(rest) = s
            .strip_prefix("export")
            .filter(|r| r.starts_with([' ', '\t']))
        {
            s = rest.trim_start();
        }
        let (key, value) = s.split_once('=').ok_or_else(invalid)?;
        let key = key.trim();
        if key.is_empty()
            || !key
                .bytes()
                .enumerate()
                .all(|(i, b)| b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit()))
        {
            return Err(invalid());
        }
        if !seen.insert(key.to_owned()) {
            bail!("duplicate variable at line {}", line + 1);
        }
        let value = value.trim_start();
        if let Some(quote) = value.chars().next().filter(|c| matches!(c, '\'' | '"')) {
            let mut chunk = &value[1..];
            let mut escaped = false;
            loop {
                let mut closing = None;
                for (i, c) in chunk.char_indices() {
                    if escaped {
                        escaped = false;
                        continue;
                    }
                    if c == '\\' && quote == '"' {
                        escaped = true;
                        continue;
                    }
                    if c == quote {
                        closing = Some(i);
                        break;
                    }
                }
                if let Some(i) = closing {
                    let tail = chunk[i + 1..].trim();
                    if !tail.is_empty() && !tail.starts_with('#') {
                        return Err(invalid());
                    }
                    break;
                }
                chunk = lines.next().ok_or_else(invalid)?.1;
                escaped = false;
            }
        }
        names.push(key.to_owned());
    }
    Ok(names)
}

pub fn preview(names: &[String]) -> String {
    let mut output = format!("{} variables\n", names.len());
    for name in names {
        output.push_str(&format!("{name}=***\n"));
    }
    output
}

pub fn check_target(path: &std::path::Path, force: bool) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                bail!("refusing symlink output target");
            }
            if !metadata.is_file() {
                bail!("output target is not a regular file");
            }
            if !force {
                bail!("output exists; use --force to overwrite");
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => bail!("could not inspect output target"),
    }
    Ok(())
}

/// The destination directory must be trusted (not writable by an adversary).
pub fn write_atomic(path: &std::path::Path, bytes: &[u8], force: bool) -> Result<()> {
    use std::io::Write;
    check_size(bytes.len())?;
    check_target(path, force)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| anyhow::anyhow!("could not create private temporary output"))?;
    #[cfg(unix)]
    temp.as_file()
        .set_permissions(std::os::unix::fs::PermissionsExt::from_mode(0o600))
        .map_err(|_| anyhow::anyhow!("could not set private permissions"))?;
    temp.write_all(bytes)
        .and_then(|()| temp.as_file().sync_all())
        .map_err(|_| anyhow::anyhow!("could not write temporary output"))?;
    check_target(path, force)?;
    if force {
        temp.persist(path)
            .map_err(|_| anyhow::anyhow!("could not atomically replace output"))?;
    } else {
        temp.persist_noclobber(path)
            .map_err(|_| anyhow::anyhow!("output exists or could not be atomically created"))?;
    }
    Ok(())
}

/// In-memory transit sink with a hard cap independent of the peer's offer.
pub struct BoundedBuffer {
    bytes: Vec<u8>,
    limit: usize,
}

impl Default for BoundedBuffer {
    fn default() -> Self {
        Self {
            bytes: Vec::new(),
            limit: MAX_PAYLOAD,
        }
    }
}

impl BoundedBuffer {
    pub fn with_limit(limit: usize) -> Result<Self> {
        check_size(limit)?;
        Ok(Self {
            bytes: Vec::new(),
            limit,
        })
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl futures_lite::io::AsyncWrite for BoundedBuffer {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        if buf.len() > self.limit - self.bytes.len() {
            return std::task::Poll::Ready(Err(std::io::Error::other(
                "payload exceeds 1 MiB limit",
            )));
        }
        self.bytes.extend_from_slice(buf);
        std::task::Poll::Ready(Ok(buf.len()))
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

/// Validate and obtain approval before creating any plaintext output.
pub fn save_received(
    bytes: &[u8],
    path: &std::path::Path,
    force: bool,
    approve: impl FnOnce(&[String]) -> Result<bool>,
) -> Result<()> {
    let names = manifest(bytes)?;
    if !approve(&names)? {
        bail!("cancelled");
    }
    write_atomic(path, bytes, force)
}
