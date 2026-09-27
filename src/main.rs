#![forbid(unsafe_code)]
#![deny(warnings)]

mod transport;
mod ui;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use envhole::{check_target, manifest, read_confirmation, read_payload};
use std::{future::Future, io, path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(version, about = "Transfer .env secrets with Magic Wormhole")]
struct Cli {
    /// Magic Wormhole rendezvous WebSocket URL
    #[arg(
        long,
        global = true,
        env = "ENVHOLE_RENDEZVOUS_URL",
        default_value = transport::DEFAULT_RENDEZVOUS_URL
    )]
    rendezvous_url: String,
    /// Magic Wormhole transit relay URL (tcp://, ws://, or wss://)
    #[arg(
        long,
        global = true,
        env = "ENVHOLE_TRANSIT_RELAY",
        default_value = transport::DEFAULT_TRANSIT_RELAY
    )]
    transit_relay: String,
    /// Maximum time for one network transfer, in seconds
    #[arg(
        long,
        global = true,
        env = "ENVHOLE_TIMEOUT_SECONDS",
        default_value_t = 600,
        value_parser = clap::value_parser!(u64).range(1..=86_400)
    )]
    timeout_seconds: u64,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Preview names and send exact bytes; use - to read stdin
    Send {
        path: PathBuf,
        /// Confirm sending without prompting
        #[arg(long)]
        yes: bool,
        /// Number of random words in the one-time code
        #[arg(
            long,
            default_value_t = transport::DEFAULT_CODE_WORDS,
            value_parser = clap::value_parser!(u8).range(2..=6)
        )]
        code_words: u8,
    },
    /// Receive, preview names, then atomically save the payload
    Receive {
        code: String,
        #[arg(long)]
        output: PathBuf,
        /// Confirm saving without prompting
        #[arg(long)]
        yes: bool,
        /// Replace an existing regular file (never a symlink)
        #[arg(long)]
        force: bool,
    },
}

fn confirm(yes: bool, stdin_payload: bool, message: &str) -> Result<()> {
    if yes {
        return Ok(());
    }
    if stdin_payload {
        bail!("stdin payload requires --yes");
    }
    ui::prompt(message)?;
    if !read_confirmation(io::stdin().lock())? {
        bail!("cancelled");
    }
    Ok(())
}

async fn with_network_timeout<T>(
    operation: impl Future<Output = Result<T>>,
    timeout: Duration,
) -> Result<T> {
    futures_lite::future::race(operation, async move {
        async_io::Timer::after(timeout).await;
        bail!("network transfer timed out")
    })
    .await
}

async fn run(cli: Cli) -> Result<()> {
    let transport = transport::Config::new(&cli.rendezvous_url, &cli.transit_relay)?;
    let network_timeout = Duration::from_secs(cli.timeout_seconds);
    match cli.command {
        Commands::Send {
            path,
            yes,
            code_words,
        } => {
            let stdin_payload = path.as_os_str() == "-";
            let bytes = if stdin_payload {
                read_payload(io::stdin().lock())?
            } else {
                read_payload(
                    std::fs::File::open(path)
                        .map_err(|_| anyhow::anyhow!("could not open input"))?,
                )?
            };
            let names = manifest(&bytes)?;
            ui::banner("Secure send");
            ui::payload_preview(&names, bytes.len());
            confirm(yes, stdin_payload, "Send this protected payload?")?;
            with_network_timeout(
                transport::send(&bytes, &transport, code_words),
                network_timeout,
            )
            .await?;
        }
        Commands::Receive {
            code,
            output,
            yes,
            force,
        } => {
            check_target(&output, force)?;
            ui::banner("Secure receive");
            let bytes =
                with_network_timeout(transport::receive(&code, &transport), network_timeout)
                    .await?;
            let payload_size = bytes.len();
            envhole::save_received(&bytes, &output, force, |names| {
                ui::payload_preview(names, payload_size);
                confirm(yes, false, "Save this protected payload?")?;
                Ok(true)
            })?;
            ui::complete(
                "SAVED",
                &format!("Saved {}", ui::format_bytes(payload_size)),
                &format!("Validated .env · {}", output.display()),
            );
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = futures_lite::future::block_on(run(Cli::parse())) {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::with_network_timeout;
    use std::{future::pending, time::Duration};

    #[test]
    fn network_timeout_stops_a_stalled_operation() {
        let result = futures_lite::future::block_on(with_network_timeout(
            pending::<anyhow::Result<()>>(),
            Duration::from_millis(5),
        ));

        assert_eq!(
            result.unwrap_err().to_string(),
            "network transfer timed out"
        );
    }

    #[test]
    fn network_timeout_returns_completed_result() {
        let result = futures_lite::future::block_on(with_network_timeout(
            async { Ok::<_, anyhow::Error>("done") },
            Duration::from_secs(1),
        ));

        assert_eq!(result.unwrap(), "done");
    }
}
