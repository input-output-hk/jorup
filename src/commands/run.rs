use crate::{
    common::JorupConfig, utils::blockchain::Blockchain, utils::release::Release,
    utils::runner::RunnerControl,
};
use semver::Version;
use structopt::StructOpt;
use thiserror::Error;

/// Run the jormungandr
#[derive(Debug, StructOpt)]
pub struct Command {
    /// The blockchain to run jormungandr for
    blockchain: String,

    /// The version of Jormungandr to run. If not specified, the latest
    /// compatible version will be used.
    #[structopt(short, long)]
    version: Option<Version>,

    /// Run the node as a daemon
    #[structopt(long)]
    daemon: bool,

    /// Extra parameters to pass on to the node
    ///
    /// Add pass on extra parameters to jormungandr for example, this command
    /// allows to change the default REST listen address, or to use a specific
    /// log formatting or output.
    extra: Vec<String>,
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
    #[error("Unable to start node")]
    Start(#[source] crate::utils::runner::Error),
}

impl Command {
    pub fn run(self, mut cfg: JorupConfig) -> Result<(), Error> {
        // prepare entry directory
        let blockchain =
            Blockchain::load(&mut cfg, &self.blockchain).map_err(Error::NoValidBlockchain)?;
        blockchain.prepare().map_err(Error::NoValidBlockchain)?;

        let release = if let Some(version) = self.version {
            Release::new(&mut cfg, version)
        } else {
            Release::load(&mut cfg, blockchain.jormungandr_version_req())
        }
        .map_err(Error::NoCompatibleRelease)?;

        if release.asset_need_fetched() {
            // asset release is not available
            return Err(Error::NoCompatibleBinaries);
        }

        let mut runner = RunnerControl::new(&blockchain, &release)
            .map_err(Error::CannotStartRunnerController)?;

        if self.daemon {
            runner.spawn(self.extra).map_err(Error::Start)
        } else {
            runner.run(self.extra).map_err(Error::Start)
        }
    }
}
