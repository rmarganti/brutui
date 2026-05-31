use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Parser, PartialEq, Eq)]
#[command(
    name = "brutui",
    version,
    about = "Terminal UI for browsing and running Bruno collections via the Bruno CLI.",
    long_about = "Brutui is a read-only Bruno collection browser and runner. It discovers Bruno collections, shows shallow request details, lets you choose collection-local environments, runs the selected root/folder/request via `bru`, and renders live output plus structured results."
)]
pub struct Cli {
    /// Path to a Bruno collection to open directly.
    #[arg(value_name = "COLLECTION_PATH")]
    pub collection_path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Clone, Subcommand, PartialEq, Eq)]
pub enum Commands {
    /// Create a config file with commented-out defaults at the platform config directory.
    Init,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn clap_command_is_constructible() {
        Cli::command().debug_assert();
    }

    #[test]
    fn collection_path_and_subcommand_parse_independently() {
        use clap::Parser;
        use super::Commands;

        let path_only = Cli::parse_from(["brutui", "/some/path"]);
        assert_eq!(path_only.collection_path, Some(std::path::PathBuf::from("/some/path")));
        assert_eq!(path_only.command, None);

        let init_only = Cli::parse_from(["brutui", "init"]);
        assert_eq!(init_only.collection_path, None);
        assert_eq!(init_only.command, Some(Commands::Init));
    }
}
