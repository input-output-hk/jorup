#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;
#[macro_use(crate_name, crate_version, crate_authors, crate_description, value_t)]
extern crate clap;
#[macro_use(lazy_static)]
extern crate lazy_static;

mod common;
mod info;
mod jorfile;
mod run;
mod setup;
mod shutdown;
mod update;
mod utils;
mod wallet;

use clap::{App, AppSettings};
use thiserror::Error;

#[derive(Debug, Error)]
enum Error {
    #[error(transparent)]
    Common(#[from] common::Error),
    #[error(transparent)]
    Update(#[from] update::Error),
    #[error(transparent)]
    Run(#[from] run::Error),
    #[error(transparent)]
    Shutdown(#[from] shutdown::Error),
    #[error(transparent)]
    Info(#[from] info::Error),
    #[error(transparent)]
    Wallet(#[from] wallet::Error),
    #[error(transparent)]
    Setup(#[from] setup::Error),
    #[error("No command given")]
    NoCommand,
    #[error("Unknown command `{0}`")]
    UnknownCommand(String),
}

fn run_main() -> Result<(), Error> {
    let mut app = App::new(crate_name!())
        .settings(&[AppSettings::ColorAuto, AppSettings::VersionlessSubcommands])
        .version(crate_version!())
        .about(crate_description!())
        .author(crate_authors!("\n"))
        .arg(common::arg::jorup_home()?)
        .arg(common::arg::generate_autocompletion())
        .arg(common::arg::jor_file())
        .arg(common::arg::offline())
        .subcommand(run::arg::command())
        .subcommand(shutdown::arg::command())
        .subcommand(info::arg::command())
        .subcommand(wallet::arg::command())
        .subcommand(setup::arg::commands())
        .subcommand(update::arg::command());

    let matches = app.clone().get_matches();

    if let Some(shell) = matches.value_of(common::arg::name::GENERATE_AUTOCOMPLETION) {
        // safe to unwrap as possible values have been validated first
        let shell = shell.parse().unwrap();

        app.gen_completions_to(crate_name!(), shell, &mut std::io::stdout());
        return Ok(());
    }

    let cfg = common::JorupConfig::new(&matches)?;

    match matches.subcommand() {
        (update::arg::name::COMMAND, matches) => update::run(cfg, matches.unwrap())?,
        (run::arg::name::COMMAND, matches) => run::run(cfg, matches.unwrap())?,
        (shutdown::arg::name::COMMAND, matches) => shutdown::run(cfg, matches.unwrap())?,
        (info::arg::name::COMMAND, matches) => info::run(cfg, matches.unwrap())?,
        (wallet::arg::name::COMMAND, matches) => wallet::run(cfg, matches.unwrap())?,
        (setup::arg::name::COMMAND, matches) => setup::run(cfg, matches.unwrap())?,
        (cmd, _) => {
            if cmd.is_empty() {
                return Err(Error::NoCommand);
            }
            return Err(Error::UnknownCommand(cmd.to_owned()));
        }
    }

    Ok(())
}

fn main() {
    use std::error::Error;

    if let Err(error) = run_main() {
        eprintln!("{}", error);
        let mut source = error.source();
        while let Some(err) = source {
            eprintln!(" |-> {}", err);
            source = err.source();
        }

        // TODO: https://github.com/rust-lang/rust/issues/43301
        //
        // as soon as #43301 is stabilized it would be nice to no use
        // `exit` but the more appropriate:
        // https://doc.rust-lang.org/stable/std/process/trait.Termination.html
        std::process::exit(1);
    }
}
