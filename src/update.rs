use crate::{
    common::JorupConfig,
    utils::{channel::Channel, download_file, release::Release},
};
use structopt::StructOpt;
use thiserror::Error;

/// Sync and update the local channel
#[derive(Debug, StructOpt)]
pub struct Command {
    /// The channel to run jormungandr for, jorup uses the default channel otherwise
    channel: Option<String>,

    /// Make the associated jormungandr release the default
    #[structopt(long)]
    make_default: bool,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error while syncing releases and channels, no internet? try `--offline`...")]
    SyncFailed(#[source] crate::common::Error),
    #[error("Cannot run the node without valid channel")]
    NoValidChannel(#[source] crate::utils::channel::Error),
    #[error("Cannot run without compatible release")]
    NoCompatibleRelease(#[source] crate::utils::release::Error),
    #[error("Cannot download and install an update")]
    CannotUpdate(#[source] crate::utils::download::Error),
    #[error("cannot save default channel")]
    CannotSaveDefaultChannel(#[source] crate::common::Error),
}

impl Command {
    pub fn run(self, mut cfg: JorupConfig) -> Result<(), Error> {
        cfg.sync_jorfile().map_err(Error::SyncFailed)?;

        // prepare entry directory
        let channel = Channel::load(&mut cfg, self.channel).map_err(Error::NoValidChannel)?;
        channel.prepare().map_err(Error::NoValidChannel)?;
        let release = Release::new(&mut cfg, channel.jormungandr_version_req())
            .map_err(Error::NoCompatibleRelease)?;
        let asset = release.asset_remote().map_err(Error::NoCompatibleRelease)?;

        if release.asset_need_fetched() && !cfg.offline() {
            download_file(
                &release.get_asset().display().to_string(),
                &asset.as_ref(),
                release.get_asset(),
            )
            .map_err(Error::CannotUpdate)?;
            println!("**** asset downloaded");
        }

        release.asset_open().map_err(Error::NoCompatibleRelease)?;

        if self.make_default {
            release
                .make_default(&cfg)
                .map_err(Error::NoCompatibleRelease)?;
            cfg.set_default_channel(channel.channel_version().clone())
                .map_err(Error::CannotSaveDefaultChannel)?;
        }

        println!(
            "**** channel {} updated to version {}",
            channel.channel_version(),
            channel.entry().channel()
        );
        println!("**** jormungandr updated to version {}", release.version());
        Ok(())
    }
}
