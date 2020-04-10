#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

mod common;
mod info;
mod jorfile;
mod run;
mod setup;
mod shutdown;
mod update;
mod utils;
mod wallet;

use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

#[derive(Debug, StructOpt)]
struct RootCmd {
    /// Set the home directory for jorup
    ///
    /// Set the directory path where jorup will install the different releases
    /// or different channels. Mainly remember to set `$JORUP_HOME/bin` value to
    /// your $PATH for easy access to the default release's tools.
    #[structopt(long)]
    jorup_home: Option<PathBuf>,

    /// Don't use the jor file from from local setting but use given one
    ///
    /// This is not to be used lightly as it may put your local jor in an
    /// invalid state. Instead of fetching the jorfile from the network and/or
    /// to use the local one, use a specific file. This is useful only for
    /// testing. This option does not imply offline.
    #[structopt(long)]
    jorfile: Option<PathBuf>,

    /// Don't query the release server to update the index
    ///
    /// Try only to work with the current states and values. Do not attempt to
    /// update the known releases and testnets. This may make your system to
    /// fail to install specific releases if they are not already cached
    /// locally.
    #[structopt(long)]
    offline: bool,

    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Generate autocompletion scripts for the given <SHELL>
    ///
    /// Generate the autocompletion scripts for the given shell, Autocompletion
    /// will be written in the standard output and can then be pasted by the
    /// user to the appropriate place.
    Completions {
        shell: structopt::clap::Shell,
    },

    Run(run::Command),
    Shutdown(shutdown::Command),
    Info(info::Command),
    Wallet(wallet::Command),
    Setup(setup::Command),
    Update(update::Command),
}

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
}

impl RootCmd {
    fn run(self) -> Result<(), Error> {
        let cfg = common::JorupConfig::new(self.jorup_home, self.jorfile, self.offline)?;

        match self.command {
            Command::Completions { shell } => Self::clap().gen_completions_to(
                env!("CARGO_PKG_NAME"),
                shell,
                &mut std::io::stdout(),
            ),
            Command::Run(cmd) => cmd.run(cfg)?,
            Command::Shutdown(cmd) => cmd.run(cfg)?,
            Command::Info(cmd) => cmd.run(cfg)?,
            Command::Wallet(cmd) => cmd.run(cfg)?,
            Command::Setup(cmd) => cmd.run(cfg)?,
            Command::Update(cmd) => cmd.run(cfg)?,
        }

        Ok(())
    }
}

fn main() {
    use std::error::Error;

    let app = RootCmd::from_args();

    if let Err(error) = app.run() {
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
