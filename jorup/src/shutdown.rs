use crate::{
    common::JorupConfig, utils::channel::Channel, utils::release::Release,
    utils::runner::RunnerControl,
};
use clap::ArgMatches;
use jorup_lib::Version;

pub mod arg {
    use crate::utils::channel::Channel;
    use clap::{App, SubCommand};

    pub mod name {
        pub const COMMAND: &str = "shutdown";
    }

    pub fn command<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name(name::COMMAND)
            .alias("stop")
            .about("Stop jormungandr")
            .arg(Channel::arg())
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
        .shutdown()
        .chain_err(|| "unable to stop/shutdown the node")
}
