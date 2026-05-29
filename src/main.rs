use anyhow::Result;
use clap::Parser;

use brutui::{app::AppBootstrap, cli::Cli};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let app = AppBootstrap::from_cli(&cli);

    app.run()
}
