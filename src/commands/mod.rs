mod blockchain;
mod defaults;
mod info;
mod node;
mod run;
mod setup;
mod shutdown;
mod wallet;

pub use setup::Install;

use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

pub trait Cmd: StructOpt {
    type Err: std::error::Error;

    fn run(self) -> Result<(), Self::Err>;
}

#[derive(Debug, StructOpt)]
pub struct RootCmd {
    /// Set the home directory for jorup
    ///
    /// Set the directory path where jorup will install the different releases
    /// or different blockchains. Mainly remember to set `$JORUP_HOME/bin` value to
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
    Blockchain(blockchain::Command),
    Node(node::Command),
    Defaults(defaults::Command),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Common(#[from] crate::common::Error),
    #[error(transparent)]
    Blockchain(#[from] blockchain::Error),
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
    #[error(transparent)]
    Node(#[from] node::Error),
    #[error(transparent)]
    Defaults(#[from] defaults::Error),
}

impl Cmd for RootCmd {
    type Err = Error;

    fn run(self) -> Result<(), Self::Err> {
        let cfg = crate::common::JorupConfig::new(self.jorup_home, self.jorfile, self.offline)?;

        if !self.offline && !matches!(self.command, Command::Setup(_)) {
            match crate::utils::check_jorup_update() {
                Ok(Some(release)) => {
                    eprintln!(
                        r#"
An update to version {} is available. Run `jorup setup update` or go to
https://input-output-hk.github.io/jorup/ to download an update."#,
                        release.version()
                    );
                }
                Err(err) => {
                    eprintln!("WARN: Could not check for jorup updates.");
                    crate::utils::print_error(err);
                }
                _ => {}
            }
        }

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
            Command::Blockchain(cmd) => cmd.run(cfg)?,
            Command::Node(cmd) => cmd.run(cfg)?,
            Command::Defaults(cmd) => cmd.run(cfg)?,
        }

        Ok(())
    }
}
