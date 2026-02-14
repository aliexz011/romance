mod commands;

use anyhow::Result;
use clap::Parser;
use commands::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    commands::run(cli)
}
