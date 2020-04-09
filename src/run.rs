use crate::{
    common::JorupConfig, utils::channel::Channel, utils::release::Release,
    utils::runner::RunnerControl,
};
use clap::ArgMatches;
use thiserror::Error;

pub mod arg {
    use crate::utils::channel::Channel;
    use clap::{App, Arg, SubCommand};

    pub mod name {
        pub const COMMAND: &str = "run";
        pub const DAEMON: &str = "DAEMON";
        pub const JORMUNGANDR: &str = "JORMUNGANDR";
        pub const JORMUNGANDR_COMMANDS: &str = "JORMUNGANDR_ADDITIONAL_OPTIONS";
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
            .arg(
                Arg::with_name(name::JORMUNGANDR_COMMANDS)
                    .last(true)
                    .help("extra parameters to pass on to the node")
                    .long_help(
                        r#"Add pass on extra parameters to jormungandr
for example, this command allows to change the default REST listen address, or
to use a specific log formatting or output"#,
                    )
                    .multiple(true),
            )
    }
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
    #[error("Cannot override jormungandr binaries")]
    CannotOverrideBinaries(#[source] crate::utils::runner::Error),
    #[error("Unable to start node")]
    Start(#[source] crate::utils::runner::Error),
}

pub fn run<'a>(mut cfg: JorupConfig, matches: &ArgMatches<'a>) -> Result<(), Error> {
    let daemon = matches.is_present(arg::name::DAEMON);

    // prepare entry directory
    let channel = Channel::load(&mut cfg, matches).map_err(Error::NoValidChannel)?;
    channel.prepare().map_err(Error::NoValidChannel)?;

    let release = Release::new(&mut cfg, channel.jormungandr_version_req())
        .map_err(Error::NoCompatibleRelease)?;

    let extra_options: Vec<_> = matches
        .values_of(arg::name::JORMUNGANDR_COMMANDS)
        .map(|c| c.collect())
        .unwrap_or_default();

    if release.asset_need_fetched() {
        // asset release is not available
        return Err(Error::NoCompatibleBinaries);
    }

    let mut runner =
        RunnerControl::new(&channel, &release).map_err(Error::CannotStartRunnerController)?;

    if let Some(jormungandr) = matches.value_of(arg::name::JORMUNGANDR) {
        runner
            .override_jormungandr(jormungandr)
            .map_err(Error::CannotOverrideBinaries)?;
    }

    if daemon {
        runner.spawn(&extra_options).map_err(Error::Start)
    } else {
        runner.run(&extra_options).map_err(Error::Start)
    }
}
