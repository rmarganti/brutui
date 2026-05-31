use anyhow::{Result, bail};
use clap::Parser;

use brutui::{
    app::AppBootstrap,
    cli::{Cli, Commands},
    config,
};

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.collection_path.is_some() && cli.command.is_some() {
        bail!("cannot specify both a collection path and a subcommand");
    }

    match cli.command {
        Some(Commands::Init) => Ok(config::init()?),
        None => AppBootstrap::from_cli(&cli).run(),
    }
}
