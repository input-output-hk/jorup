use clap::{App, Arg, SubCommand};

pub mod name {
    pub const COMMAND: &str = "testnet";
    pub const COMMAND_ADD: &str = "add";
    pub const CHANNEL_NAME: &str = "CHANNEL";
    pub const VERSION_REQ: &str = "JORMUNGANDR_VERSION";
    pub const DESCRIPTION: &str = "DESCRIPTION";
    pub const DISPOSITION: &str = "DISPOSITION";
    pub const GENESIS_FILE: &str = "GENESIS_FILE";
}

mod validator {
    use std::str::FromStr as _;

    pub fn partial_channel_desc(arg: String) -> Result<(), String> {
        use error_chain::ChainedError as _;
        use jorup_lib::PartialChannelDesc;

        PartialChannelDesc::from_str(&arg)
            .map(|_channel| ())
            .map_err(|err| err.display_chain().to_string())
    }

    pub fn version_req(arg: String) -> Result<(), String> {
        use jorup_lib::VersionReq;

        VersionReq::from_str(&arg)
            .map(|_version| ())
            .map_err(|err| err.to_string())
    }
}

pub fn command<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name(&name::COMMAND)
        .about("Testnet operations: add, dispose or remove")
        .subcommand(command_add())
}

fn command_add<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name(name::COMMAND_ADD)
        .about("Add a new testnet")
        .arg(
            Arg::with_name(name::CHANNEL_NAME)
                .required(true)
                .value_name(name::CHANNEL_NAME)
                .help("the channel to create")
                .validator(validator::partial_channel_desc),
        )
        .arg(
            Arg::with_name(name::VERSION_REQ)
                .long("version-req")
                .value_name(name::VERSION_REQ)
                .help("the version requirements of the supported jormungandr")
                .validator(validator::version_req),
        )
        .arg(
            Arg::with_name(name::DESCRIPTION)
                .long("description")
                .value_name(name::DESCRIPTION)
                .help("short description of what this is for"),
        )
        .arg(
            Arg::with_name(name::DISPOSITION)
                .long("disposition")
                .value_name(name::DISPOSITION)
                .help("initial disposition of this testnet")
                .default_value("up")
                .possible_values(&["up", "down"]),
        )
        .arg(
            Arg::with_name(name::GENESIS_FILE)
                .long("genesis-file")
                .value_name(name::GENESIS_FILE)
                .help("file path to the genesis file to use for this testnet"),
        )
}
