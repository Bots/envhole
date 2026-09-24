use anyhow::{Result, anyhow, bail};
use envhole::{BoundedBuffer, MAX_PAYLOAD};
use magic_wormhole::{MailboxConnection, Wormhole, transfer, transit};
use std::{future::pending, io::Write};

fn relay_hints() -> Result<Vec<transit::RelayHint>> {
    Ok(vec![transit::RelayHint::from_urls(
        None,
        [transit::DEFAULT_RELAY_SERVER.parse()?],
    )?])
}

// Do not expose library errors: peer-controlled messages can contain secret text.
pub async fn send(bytes: &[u8]) -> Result<()> {
    let mailbox = MailboxConnection::create(transfer::APP_CONFIG, 2)
        .await
        .map_err(|_| anyhow!("could not create rendezvous connection"))?;
    println!("Code: {}", mailbox.code());
    std::io::stdout().flush()?;
    let wormhole = Wormhole::connect(mailbox)
        .await
        .map_err(|_| anyhow!("wormhole connection or authentication failed"))?;
    transfer::send_file(
        wormhole,
        relay_hints()?,
        &mut futures_lite::io::Cursor::new(bytes),
        "envhole.env",
        bytes.len() as u64,
        transit::Abilities::ALL,
        |_| {},
        |_, _| {},
        pending(),
    )
    .await
    .map_err(|_| anyhow!("encrypted transfer failed"))?;
    Ok(())
}

pub async fn receive(code: &str) -> Result<Vec<u8>> {
    let code = code.parse().map_err(|_| anyhow!("invalid wormhole code"))?;
    let mailbox = MailboxConnection::connect(transfer::APP_CONFIG, code, false)
        .await
        .map_err(|_| anyhow!("could not join rendezvous; check code and network"))?;
    let wormhole = Wormhole::connect(mailbox)
        .await
        .map_err(|_| anyhow!("wormhole connection or authentication failed"))?;
    let request =
        transfer::request_file(wormhole, relay_hints()?, transit::Abilities::ALL, pending())
            .await
            .map_err(|_| anyhow!("could not receive file offer"))?
            .ok_or_else(|| anyhow!("transfer cancelled"))?;
    let expected = request.file_size();
    if expected > MAX_PAYLOAD as u64 {
        let _ = request.reject().await;
        bail!("payload exceeds 1 MiB limit");
    }
    let mut buffer = BoundedBuffer::with_limit(expected as usize)?;
    request
        .accept(|_| {}, |_, _| {}, &mut buffer, pending())
        .await
        .map_err(|_| anyhow!("encrypted transfer failed (payload limit 1 MiB)"))?;
    let bytes = buffer.into_bytes();
    if bytes.len() as u64 != expected {
        bail!("payload length mismatch");
    }
    Ok(bytes)
}
