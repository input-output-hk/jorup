use crate::{
    common::JorupConfig, utils::channel::Channel, utils::release::Release,
    utils::runner::RunnerControl,
};
use structopt::StructOpt;
use thiserror::Error;

/// Wallet operations
#[derive(Debug, StructOpt)]
pub struct Command {
    /// The channel to run jormungandr for, jorup uses the default channel otherwise
    channel: Option<String>,

    /// Force re-creating a wallet if it does exists already
    #[structopt(long)]
    force_create_wallet: bool,
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
    #[error("Cannot create new wallet")]
    CannotCreateWallet(#[source] crate::utils::runner::Error),
    #[error("Cannot get the wallet's address")]
    CannotGetAddress(#[source] crate::utils::runner::Error),
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

        runner
            .get_wallet_secret_key(self.force_create_wallet)
            .map_err(Error::CannotCreateWallet)?;
        let address = runner
            .get_wallet_address()
            .map_err(Error::CannotGetAddress)?;

        println!("Wallet: {}", address);

        Ok(())
    }
}
