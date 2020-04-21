use crate::{
    common::JorupConfig, utils::blockchain::Blockchain, utils::release::Release,
    utils::runner::RunnerControl,
};
use structopt::StructOpt;
use thiserror::Error;

/// Stop jormungandr
#[derive(Debug, StructOpt)]
pub struct Command {
    /// The blockchain to run jormungandr for
    blockchain: String,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot run the node without valid blockchain")]
    NoValidBlockchain(#[source] crate::utils::blockchain::Error),
    #[error("Cannot run without compatible release")]
    NoCompatibleRelease(#[source] crate::utils::release::Error),
    #[error("No binaries for this blockchain")]
    NoCompatibleBinaries,
    #[error("Unable to start the runner controller")]
    CannotStartRunnerController(#[source] crate::utils::runner::Error),
    #[error("unable to stop/shutdown the node")]
    ShutdownError(#[source] crate::utils::runner::Error),
}

impl Command {
    pub fn run(self, mut cfg: JorupConfig) -> Result<(), Error> {
        // prepare entry directory
        let blockchain =
            Blockchain::load(&mut cfg, &self.blockchain).map_err(Error::NoValidBlockchain)?;
        blockchain.prepare().map_err(Error::NoValidBlockchain)?;

        let release = Release::new(&mut cfg, blockchain.jormungandr_version_req())
            .map_err(Error::NoCompatibleRelease)?;

        if release.asset_need_fetched() {
            // asset release is not available
            return Err(Error::NoCompatibleBinaries);
        }

        let mut runner = RunnerControl::new(&blockchain, &release)
            .map_err(Error::CannotStartRunnerController)?;

        runner.shutdown().map_err(Error::ShutdownError)
    }
}
