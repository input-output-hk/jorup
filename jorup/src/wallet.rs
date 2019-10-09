use crate::{
    common::JorupConfig, utils::channel::Channel, utils::release::Release,
    utils::runner::RunnerControl,
};
use clap::ArgMatches;
use jorup_lib::Version;

pub mod arg {
    use crate::utils::channel::Channel;
    use clap::{App, Arg, SubCommand};

    pub mod name {
        pub const COMMAND: &str = "wallet";
    }

    pub fn command<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name(name::COMMAND)
            .about("wallet operations")
            .arg(Channel::arg())
            .arg(
                Arg::with_name("FORCE_CREATE_WALLET")
                    .long("force-create")
                    .alias("force")
                    .help("force re-creating a wallet if it does not exists already"),
            )
    }
}

error_chain! {
    errors {
        Release (version: Version) {
            description("Error with the release"),
            display("Error with release: {}", version),
        }
    }
}

pub fn run<'a>(mut cfg: JorupConfig, matches: &ArgMatches<'a>) -> Result<()> {
    cfg.sync_jorfile().chain_err(|| {
        "Error while syncing releases and channels, no internet? try `--offline`..."
    })?;

    let force_new = matches.is_present("FORCE_CREATE_WALLET");

    // prepare entry directory
    let channel = Channel::load(&mut cfg, matches)
        .chain_err(|| "Cannot run the node without valid channel")?;
    channel
        .prepare()
        .chain_err(|| "Cannot run the node without valid channel")?;

    let release = Release::new(&mut cfg, channel.jormungandr_version_req())
        .chain_err(|| "Cannot run without compatible release")?;

    if release.asset_need_fetched() {
        // asset release is not available
        bail!(
            "No binaries for this channel, run `jorup update {}`",
            channel.channel_version()
        );
    }

    release
        .asset_open()
        .chain_err(|| ErrorKind::Release(release.version().clone()))?;

    let mut runner = RunnerControl::new(&channel, &release)
        .chain_err(|| "Unable to start the runner controller")?;

    runner
        .get_wallet_secret_key(force_new)
        .chain_err(|| "Cannot create new wallet")?;

    let address = runner
        .get_wallet_address()
        .chain_err(|| "Cannot get the wallet's address")?;

    println!("Wallet: {}", address);

    Ok(())
}
