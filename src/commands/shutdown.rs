use crate::{
    common::JorupConfig, utils::channel::Channel, utils::release::Release,
    utils::runner::RunnerControl,
};
use structopt::StructOpt;
use thiserror::Error;

/// Stop jormungandr
#[derive(Debug, StructOpt)]
pub struct Command {
    /// The channel to run jormungandr for, jorup uses the default channel otherwise
    channel: Option<String>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot run the node without valid channel")]
    NoValidChannel(#[source] crate::utils::channel::Error),
    #[error("Cannot run without compatible release")]
    NoCompatibleRelease(#[source] crate::utils::release::Error),
    #[error("No binaries for this channel")]
    NoCompatibleBinaries,
    #[error("Unable to start the runner controller")]
    CannotStartRunnerController(#[source] crate::utils::runner::Error),
    #[error("unable to stop/shutdown the node")]
    ShutdownError(#[source] crate::utils::runner::Error),
}

impl Command {
    pub fn run(self, mut cfg: JorupConfig) -> Result<(), Error> {
        // prepare entry directory
        let channel = Channel::load(&mut cfg, self.channel).map_err(Error::NoValidChannel)?;
        channel.prepare().map_err(Error::NoValidChannel)?;

        let release = Release::new(&mut cfg, channel.jormungandr_version_req())
            .map_err(Error::NoCompatibleRelease)?;

        if release.asset_need_fetched() {
            // asset release is not available
            return Err(Error::NoCompatibleBinaries);
        }

        let mut runner =
            RunnerControl::new(&channel, &release).map_err(Error::CannotStartRunnerController)?;

        runner.shutdown().map_err(Error::ShutdownError)
    }
}
