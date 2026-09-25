use crate::ui;
use anyhow::{Result, anyhow, bail};
use envhole::{BoundedBuffer, MAX_PAYLOAD};
use magic_wormhole::{AppConfig, MailboxConnection, Wormhole, transfer, transit};
use std::{borrow::Cow, future::pending};

pub const DEFAULT_RENDEZVOUS_URL: &str = magic_wormhole::rendezvous::DEFAULT_RENDEZVOUS_SERVER;
pub const DEFAULT_TRANSIT_RELAY: &str = transit::DEFAULT_RELAY_SERVER;

pub struct Config {
    app: AppConfig<transfer::AppVersion>,
    relays: Vec<transit::RelayHint>,
    rendezvous_url: String,
    transit_relay: String,
}

impl Config {
    pub fn new(rendezvous_url: &str, transit_relay: &str) -> Result<Self> {
        if rendezvous_url.chars().any(char::is_control) {
            bail!("invalid rendezvous URL: control characters are not allowed");
        }
        let rendezvous = rendezvous_url
            .parse::<url::Url>()
            .map_err(|_| anyhow!("invalid rendezvous URL"))?;
        if !matches!(rendezvous.scheme(), "ws" | "wss") || rendezvous.host_str().is_none() {
            bail!("invalid rendezvous URL: expected ws:// or wss:// with a host");
        }

        if transit_relay.chars().any(char::is_control) {
            bail!("invalid transit relay URL: control characters are not allowed");
        }
        let relay_url = transit_relay
            .parse::<url::Url>()
            .map_err(|_| anyhow!("invalid transit relay URL"))?;
        let relay = transit::RelayHint::from_urls(None, [relay_url])
            .map_err(|_| anyhow!("invalid transit relay URL"))?;
        let app = transfer::APP_CONFIG
            .clone()
            .rendezvous_url(Cow::Owned(rendezvous.to_string()));
        Ok(Self {
            app,
            relays: vec![relay],
            rendezvous_url: rendezvous_url.to_owned(),
            transit_relay: transit_relay.to_owned(),
        })
    }

    fn receive_command(&self, code: &str) -> String {
        format!(
            "ENVHOLE_RENDEZVOUS_URL={} ENVHOLE_TRANSIT_RELAY={} \
envhole receive {} --output .env.received",
            shell_quote(&self.rendezvous_url),
            shell_quote(&self.transit_relay),
            shell_quote(code),
        )
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

// Do not expose library errors: peer-controlled messages can contain secret text.
pub async fn send(bytes: &[u8], config: &Config) -> Result<()> {
    ui::step("Connecting to the rendezvous server…");
    let mailbox = MailboxConnection::create(config.app.clone(), 2)
        .await
        .map_err(|_| anyhow!("could not create rendezvous connection"))?;
    let code = mailbox.code().to_string();
    ui::success("One-time code created");
    ui::plain(&format!("Code: {code}"));
    ui::output_block(&ui::receiver_command_box(&config.receive_command(&code)));
    ui::output_block(&ui::encrypted_payload_notice(bytes.len()));
    ui::step("Waiting for the receiving machine…");
    let wormhole = Wormhole::connect(mailbox)
        .await
        .map_err(|_| anyhow!("wormhole connection or authentication failed"))?;
    ui::secure("Peer authenticated · encrypted channel established");
    ui::step("Negotiating a direct connection or encrypted relay…");
    transfer::send_file(
        wormhole,
        config.relays.clone(),
        &mut futures_lite::io::Cursor::new(bytes),
        "envhole.env",
        bytes.len() as u64,
        transit::Abilities::ALL,
        |info| ui::success(&format!("Transit connected {}", info.conn_type)),
        |_, _| {},
        pending(),
    )
    .await
    .map_err(|_| anyhow!("encrypted transfer failed"))?;
    ui::success(&format!(
        "Delivered {} through the encrypted channel",
        ui::format_bytes(bytes.len())
    ));
    Ok(())
}

pub async fn receive(code: &str, config: &Config) -> Result<Vec<u8>> {
    let code = code.parse().map_err(|_| anyhow!("invalid wormhole code"))?;
    ui::step("Joining the rendezvous with the one-time code…");
    let mailbox = MailboxConnection::connect(config.app.clone(), code, false)
        .await
        .map_err(|_| anyhow!("could not join rendezvous; check code and network"))?;
    ui::success("Rendezvous joined");
    let wormhole = Wormhole::connect(mailbox)
        .await
        .map_err(|_| anyhow!("wormhole connection or authentication failed"))?;
    ui::secure("Sender authenticated · encrypted channel established");
    ui::step("Waiting for the encrypted file offer…");
    let request = transfer::request_file(
        wormhole,
        config.relays.clone(),
        transit::Abilities::ALL,
        pending(),
    )
    .await
    .map_err(|_| anyhow!("could not receive file offer"))?
    .ok_or_else(|| anyhow!("transfer cancelled"))?;
    let expected = request.file_size();
    if expected > MAX_PAYLOAD as u64 {
        let _ = request.reject().await;
        bail!("payload exceeds 1 MiB limit");
    }
    ui::output_block(&ui::encrypted_payload_notice(expected as usize));
    let mut buffer = BoundedBuffer::with_limit(expected as usize)?;
    ui::step("Receiving and verifying encrypted bytes…");
    request
        .accept(
            |info| ui::success(&format!("Transit connected {}", info.conn_type)),
            |_, _| {},
            &mut buffer,
            pending(),
        )
        .await
        .map_err(|_| anyhow!("encrypted transfer failed (payload limit 1 MiB)"))?;
    let bytes = buffer.into_bytes();
    if bytes.len() as u64 != expected {
        bail!("payload length mismatch");
    }
    ui::success(&format!(
        "Received and verified {} from the encrypted channel",
        ui::format_bytes(bytes.len())
    ));
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::{Config, DEFAULT_RENDEZVOUS_URL, DEFAULT_TRANSIT_RELAY};

    #[test]
    fn receive_command_contains_complete_shell_safe_configuration() {
        let config = Config::new("ws://192.168.1.216:4000/v1", "tcp://192.168.1.216:4001").unwrap();

        assert_eq!(
            config.receive_command("7-example-words"),
            "ENVHOLE_RENDEZVOUS_URL='ws://192.168.1.216:4000/v1' \
ENVHOLE_TRANSIT_RELAY='tcp://192.168.1.216:4001' \
envhole receive '7-example-words' --output .env.received"
        );
    }

    #[test]
    fn receive_command_quotes_shell_metacharacters() {
        let config = Config::new(
            "wss://example.com/v1?label=sender's",
            "tcp://relay.example.com:4001",
        )
        .unwrap();
        let command = config.receive_command("7-code'word");

        assert!(command.contains("sender'\"'\"'s"));
        assert!(command.contains("7-code'\"'\"'word"));
    }

    #[test]
    fn config_rejects_terminal_control_characters() {
        assert!(Config::new("ws://example.com/v1\nBAD", DEFAULT_TRANSIT_RELAY).is_err());
        assert!(Config::new(DEFAULT_RENDEZVOUS_URL, "tcp://example.com:4001\rBAD").is_err());
    }
}
