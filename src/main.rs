#![forbid(unsafe_code)]
#![deny(warnings)]

mod transport;
mod ui;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use envhole::{check_target, manifest, read_confirmation, read_payload};
use std::{io, path::PathBuf};

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

async fn run(cli: Cli) -> Result<()> {
    let transport = transport::Config::new(&cli.rendezvous_url, &cli.transit_relay)?;
    match cli.command {
        Commands::Send { path, yes } => {
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
            transport::send(&bytes, &transport).await?;
        }
        Commands::Receive {
            code,
            output,
            yes,
            force,
        } => {
            check_target(&output, force)?;
            ui::banner("Secure receive");
            let bytes = transport::receive(&code, &transport).await?;
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
