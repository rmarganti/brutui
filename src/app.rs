use anyhow::{Result, bail};

use crate::cli::Cli;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupMode {
    CollectionPathProvided,
    AutomaticDiscovery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppBootstrap {
    pub mode: StartupMode,
}

impl AppBootstrap {
    pub fn from_cli(cli: &Cli) -> Self {
        let mode = if cli.collection_path.is_some() {
            StartupMode::CollectionPathProvided
        } else {
            StartupMode::AutomaticDiscovery
        };

        Self { mode }
    }

    pub fn run(self) -> Result<()> {
        let mode = match self.mode {
            StartupMode::CollectionPathProvided => "explicit collection path",
            StartupMode::AutomaticDiscovery => "automatic collection discovery",
        };

        bail!(
            "Brutui scaffold complete, but startup flow for {mode} is not implemented yet. Follow-on ishes will add discovery, TUI rendering, and Bruno execution."
        )
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::cli::Cli;

    use super::{AppBootstrap, StartupMode};

    #[test]
    fn bootstrap_detects_automatic_discovery_mode() {
        let cli = Cli::parse_from(["brutui"]);

        let bootstrap = AppBootstrap::from_cli(&cli);

        assert_eq!(bootstrap.mode, StartupMode::AutomaticDiscovery);
    }

    #[test]
    fn bootstrap_detects_explicit_collection_mode() {
        let cli = Cli::parse_from(["brutui", "fixtures/demo"]);

        let bootstrap = AppBootstrap::from_cli(&cli);

        assert_eq!(bootstrap.mode, StartupMode::CollectionPathProvided);
    }
}
