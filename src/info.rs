use crate::{
    common::JorupConfig, utils::channel::Channel, utils::release::Release,
    utils::runner::RunnerControl,
};
use clap::ArgMatches;
use thiserror::Error;

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
    #[error("Cannot collect node's info")]
    CannotCollectInfo(#[source] crate::utils::runner::Error),
}

pub mod arg {
    use crate::utils::channel::Channel;
    use clap::{App, SubCommand};

    pub mod name {
        pub const COMMAND: &str = "info";
    }

    pub fn command<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name(name::COMMAND)
            .about("get running node's info")
            .arg(Channel::arg())
    }
}

pub fn run<'a>(mut cfg: JorupConfig, matches: &ArgMatches<'a>) -> Result<(), Error> {
    // prepare entry directory
    let channel = Channel::load(&mut cfg, matches).map_err(Error::NoValidChannel)?;
    channel.prepare().map_err(Error::NoValidChannel)?;

    let release = Release::new(&mut cfg, channel.jormungandr_version_req())
        .map_err(Error::NoCompatibleRelease)?;

    if release.asset_need_fetched() {
        // asset release is not available
        return Err(Error::NoCompatibleBinaries);
    }

    let mut runner =
        RunnerControl::new(&channel, &release).map_err(Error::CannotStartRunnerController)?;

    runner.settings().map_err(Error::CannotCollectInfo)?;
    runner.info().map_err(Error::CannotCollectInfo)
}
