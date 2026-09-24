#![forbid(unsafe_code)]
#![deny(warnings)]

mod transport;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use envhole::{check_target, manifest, preview, read_confirmation, read_payload};
use std::{
    io::{self, Write},
    path::PathBuf,
};

#[derive(Parser)]
#[command(version, about = "Transfer .env secrets with Magic Wormhole")]
struct Cli {
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

fn confirm(yes: bool, stdin_payload: bool) -> Result<()> {
    if yes {
        return Ok(());
    }
    if stdin_payload {
        bail!("stdin payload requires --yes");
    }
    eprint!("Continue? [y/N] ");
    io::stderr().flush()?;
    if !read_confirmation(io::stdin().lock())? {
        bail!("cancelled");
    }
    Ok(())
}

async fn run(cli: Cli) -> Result<()> {
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
            print!("{}", preview(&manifest(&bytes)?));
            io::stdout().flush()?;
            confirm(yes, stdin_payload)?;
            transport::send(&bytes).await?;
        }
        Commands::Receive {
            code,
            output,
            yes,
            force,
        } => {
            check_target(&output, force)?;
            let bytes = transport::receive(&code).await?;
            envhole::save_received(&bytes, &output, force, |names| {
                print!("{}", preview(names));
                io::stdout().flush()?;
                confirm(yes, false)?;
                Ok(true)
            })?;
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
