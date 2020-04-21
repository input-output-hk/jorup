use crate::{
    common::JorupConfig, utils::blockchain::Blockchain, utils::release::Release,
    utils::runner::RunnerControl,
};
use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

/// Run the jormungandr
#[derive(Debug, StructOpt)]
pub struct Command {
    /// The blockchain to run jormungandr for
    blockchain: String,

    /// Run the node as a daemon
    #[structopt(long)]
    daemon: bool,

    /// Use this specified binary as executable
    #[structopt(long)]
    jormungandr: Option<PathBuf>,

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
    #[error("Cannot override jormungandr binaries")]
    CannotOverrideBinaries(#[source] crate::utils::runner::Error),
    #[error("Unable to start node")]
    Start(#[source] crate::utils::runner::Error),
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

        if let Some(jormungandr) = self.jormungandr {
            runner
                .override_jormungandr(jormungandr)
                .map_err(Error::CannotOverrideBinaries)?;
        }

        if self.daemon {
            runner.spawn(self.extra).map_err(Error::Start)
        } else {
            runner.run(self.extra).map_err(Error::Start)
        }
    }
}
