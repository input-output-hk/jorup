use crate::{
    common::JorupConfig,
    utils::{blockchain::Blockchain, runner::RunnerControl},
};
use structopt::StructOpt;
use thiserror::Error;

/// Get running node's info
#[derive(Debug, StructOpt)]
pub struct Command {
    /// The blockchain to run jormungandr for
    blockchain: String,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot run the node without valid blockchain")]
    NoValidBlockchain(#[source] crate::utils::blockchain::Error),
    #[error("Unable to start the runner controller")]
    CannotStartRunnerController(#[source] crate::utils::runner::Error),
    #[error("Cannot collect node's info")]
    CannotCollectInfo(#[source] crate::utils::runner::Error),
}

impl Command {
    pub fn run(self, mut cfg: JorupConfig) -> Result<(), Error> {
        let blockchain =
            Blockchain::load(&mut cfg, &self.blockchain).map_err(Error::NoValidBlockchain)?;
        blockchain.prepare().map_err(Error::NoValidBlockchain)?;

        let mut runner =
            RunnerControl::load(&blockchain).map_err(Error::CannotStartRunnerController)?;

        runner.settings().map_err(Error::CannotCollectInfo)?;
        runner.info().map_err(Error::CannotCollectInfo)
    }
}
