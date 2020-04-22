use crate::common::JorupConfig;
use structopt::StructOpt;
use thiserror::Error;

/// Blockchain configuration management
#[derive(Debug, StructOpt)]
pub enum Command {
    /// Download the latest config for blockchains
    Update,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error while syncing releases and blockchains, no internet? try `--offline`...")]
    SyncFailed(#[source] crate::common::Error),
}

impl Command {
    pub fn run(self, cfg: JorupConfig) -> Result<(), Error> {
        match self {
            Command::Update => {
                cfg.sync_jorfile().map_err(Error::SyncFailed)?;
            }
        }
        Ok(())
    }
}
