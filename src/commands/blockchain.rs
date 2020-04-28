use crate::common::JorupConfig;
use structopt::StructOpt;
use thiserror::Error;

/// Blockchain configuration management
#[derive(Debug, StructOpt)]
pub enum Command {
    /// Download the latest config for blockchains
    Update,
    /// List blockchains from `jorfile.json`
    List,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error while syncing releases and blockchains, no internet? try `--offline`...")]
    SyncFailed(#[source] crate::common::Error),
    #[error("Failed to load jorfile.json")]
    JorfileLoadFailed(#[source] crate::common::Error),
}

impl Command {
    pub fn run(self, mut cfg: JorupConfig) -> Result<(), Error> {
        match self {
            Command::Update => {
                cfg.sync_jorfile().map_err(Error::SyncFailed)?;
            }
            Command::List => {
                let config = cfg.load_jor().map_err(Error::JorfileLoadFailed)?;
                for blockchain in config.blockchains().iter() {
                    println!("\t{}\n{}\n", blockchain.name(), blockchain.description());
                }
            }
        }
        Ok(())
    }
}
