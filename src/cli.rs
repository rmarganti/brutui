use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser, PartialEq, Eq)]
#[command(
    name = "brutui",
    version,
    about = "Terminal UI for browsing and running Bruno collections via the Bruno CLI.",
    long_about = "Brutui is a read-only Bruno collection browser and runner. v0.1 scaffolds the core architecture while later work adds discovery, inspection, environments, and execution."
)]
pub struct Cli {
    #[arg(value_name = "COLLECTION_PATH")]
    pub collection_path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn clap_command_is_constructible() {
        Cli::command().debug_assert();
    }
}
