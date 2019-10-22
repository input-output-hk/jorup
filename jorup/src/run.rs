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
        pub const COMMAND: &str = "run";
        pub const DAEMON: &str = "DAEMON";
        pub const JORMUNGANDR: &str = "JORMUNGANDR";
    }

    pub fn command<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name(name::COMMAND)
            .about("Run the jormungandr")
            .arg(Channel::arg())
            .arg(
                Arg::with_name(name::DAEMON)
                    .long("daemon")
                    .help("Run the node as a daemon"),
            )
            .arg(
                Arg::with_name(name::JORMUNGANDR)
                    .long("jormungandr")
                    .takes_value(true)
                    .value_name("PATH")
                    .help("use this specified binary as executable")
                    .hidden(true),
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
    let daemon = matches.is_present(arg::name::DAEMON);

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

    if let Some(jormungandr) = matches.value_of(arg::name::JORMUNGANDR) {
        runner
            .override_jormungandr(jormungandr)
            .chain_err(|| "Cannot override jormungandr binaries")?;
    }

    if daemon {
        runner.spawn().chain_err(|| "Unable to start the node")
    } else {
        runner.run().chain_err(|| "Unable to start the node")
    }
}
