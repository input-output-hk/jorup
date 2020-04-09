use crate::{
    common::JorupConfig,
    utils::{channel::Channel, download_file, release::Release},
};
use clap::ArgMatches;
use thiserror::Error;

pub mod arg {
    use crate::utils::channel::Channel;
    use clap::{App, Arg, SubCommand};

    pub mod name {
        pub const COMMAND: &str = "update";
        pub const MAKE_DEFAULT: &str = "MAKE_DEFAULT";
    }

    pub fn command<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name(name::COMMAND)
            .about("sync and update the local channel")
            .arg(Channel::arg())
            .arg(
                Arg::with_name(name::MAKE_DEFAULT)
                    .long("default")
                    .help("make the associated jormungandr release the default"),
            )
    }
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

pub fn run<'a>(mut cfg: JorupConfig, matches: &ArgMatches<'a>) -> Result<(), Error> {
    cfg.sync_jorfile().map_err(Error::SyncFailed)?;

    let make_default = matches.is_present(arg::name::MAKE_DEFAULT);

    // prepare entry directory
    let channel = Channel::load(&mut cfg, matches).map_err(Error::NoValidChannel)?;
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

    if make_default {
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
