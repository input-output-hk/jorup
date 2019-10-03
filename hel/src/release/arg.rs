use clap::{App, Arg, SubCommand};

pub mod name {
    pub const COMMAND: &str = "release";
    pub const COMMAND_ADD: &str = "add";
    pub const RELEASE_NAME: &str = "RELEASE";
}

mod validator {
    use std::str::FromStr as _;

    pub fn release(arg: String) -> Result<(), String> {
        use jorup_lib::Version;

        Version::from_str(&arg)
            .map(|_channel| ())
            .map_err(|err| err.to_string())
    }
}

pub fn command<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name(&name::COMMAND)
        .about("Release operations: add or remove")
        .subcommand(command_add())
}

fn command_add<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name(name::COMMAND_ADD)
        .about("Add a new release")
        .arg(
            Arg::with_name(name::RELEASE_NAME)
                .required(true)
                .value_name(name::RELEASE_NAME)
                .help("the channel to create")
                .validator(validator::release),
        )
}
