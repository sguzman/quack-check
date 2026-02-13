use anyhow::Result;
use clap::Parser;
use quack_check::cli;
use tracing::error;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    if let Err(err) = cli::dispatch(args) {
        error!("{:#}", err);
        std::process::exit(1);
    }
    Ok(())
}
