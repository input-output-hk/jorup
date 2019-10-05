use crate::{channel::Channel, common::JorupConfig, release::Release};
use clap::ArgMatches;
use jorup_lib::{Version, VersionReq};
use tokio::prelude::*;
use tokio_process::CommandExt as _;

pub mod arg {
    use clap::{App, Arg, SubCommand};

    pub mod name {
        pub const COMMAND: &str = "run";
        pub const CHANNEL_NAME: &str = "CHANNEL";
    }

    pub fn command<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name(name::COMMAND)
            .about("Run the jormungandr")
            .arg(
                Arg::with_name(name::CHANNEL_NAME)
                    .value_name(name::CHANNEL_NAME)
                    .help("The channel to run jormungandr for, jorup uses the default channel otherwise")
                    .validator(validator::channel),
            )
    }

    mod validator {
        use std::str::FromStr as _;

        pub fn channel(arg: String) -> Result<(), String> {
            use crate::common::Channel;
            use error_chain::ChainedError as _;

            Channel::from_str(&arg)
                .map(|_channel| ())
                .map_err(|err| err.display_chain().to_string())
        }
    }
}

error_chain! {
    errors {
        Channel (channel: jorup_lib::Channel) {
            description("Error with the channel"),
            display("Error with channel: {}", channel),
        }

        Release (version: Version) {
            description("Error with the release"),
            display("Error with release: {}", version),
        }
    }
}

pub fn run<'a>(cfg: JorupConfig, matches: &ArgMatches<'a>) -> Result<()> {
    cfg.sync_jorfile().chain_err(|| {
        "Error while syncing releases and channels, no internet? try `--offline`..."
    })?;

    let jor = cfg
        .load_jor()
        .chain_err(|| "No jorfile... cannot operate")?;

    let mut channel_entered = cfg.current_channel().clone();

    let entry = if let Some(channel) = matches.value_of(arg::name::CHANNEL_NAME) {
        // should be save to unwrap as we have set a validator in the Argument
        // for the CLI to check it is valid
        use crate::common::Channel::*;
        channel_entered = channel.parse().unwrap();
        match channel.parse().unwrap() {
            Nightly => jor.search_entry(true, VersionReq::any()),
            Stable => jor.search_entry(false, VersionReq::any()),
            Specific { channel } => jor.entries().get(&channel),
        }
    } else {
        cfg.current_entry(&jor)
    };

    let entry = entry.ok_or(Error::from("channel does not exist"))?;

    // prepare entry directory
    let channel = Channel::new(&cfg, entry.clone())
        .chain_err(|| ErrorKind::Channel(entry.channel().clone()))?;
    channel
        .prepare()
        .chain_err(|| ErrorKind::Channel(entry.channel().clone()))?;
    let release =
        if let Some(release) = jor.search_release(channel.entry().jormungandr_versions().clone()) {
            Release::new(&cfg, release.clone())
                .chain_err(|| ErrorKind::Release(release.version().clone()))?
        } else {
            bail!("No release for this channel")
        };

    if release.asset_need_fetched() {
        // asset release is not available
        bail!(
            "No binaries for this channel, run `jorup update {}`",
            channel_entered
        );
    }

    release
        .asset_open()
        .chain_err(|| ErrorKind::Release(release.version().clone()))?;

    let mut cmd = std::process::Command::new(release.get_jormungandr());

    cmd.current_dir(channel.dir());

    cmd.args(&[
        "--genesis-block",
        channel.get_genesis_block().display().to_string().as_str(),
    ]);

    for peer in entry.known_trusted_peers() {
        cmd.args(&["--trusted-peer", peer.to_string().as_str()]);
    }

    let child = cmd.spawn_async().chain_err(|| "Cannot start jormungandr")?;

    tokio::run(
        child
            .map(|status| println!("exit status: {}", status))
            .map_err(|e| panic!("failed to wait for exit: {}", e)),
    );

    Ok(())
}
