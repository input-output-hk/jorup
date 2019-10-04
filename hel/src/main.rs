#[macro_use(crate_name, crate_version, crate_authors, crate_description, values_t)]
extern crate clap;
#[macro_use(error_chain, bail, quick_main)]
extern crate error_chain;

mod common;
mod release;
mod testnet;

use clap::App;
use std::path::PathBuf;

error_chain! {
    foreign_links {
        Clap(clap::Error);
        Io(std::io::Error);
    }

    links {
        Testnet(testnet::Error, testnet::ErrorKind);
        Release(release::Error, release::ErrorKind);
    }

    errors {
        NoCommand {
            description("No commands, try '--help' for more information")
        }

        UnknownCommand (cmd: String) {
            description("Unknown command"),
            display("Unknown command '{}', try '--help' to see full list of commands", cmd),
        }

        ExpectedOptionsOrSubCommands (cmd: String) {
            description("Command expected options or sub-commands"),
            display("Command '{}' expected more options, try '{} --help' to find out more", cmd, cmd),
        }

        MissingReleaseFile {
            description("Missing release file"),
            display("Missing release file, add '--file <FILE>' where <FILE> is the path to the file with the release info, see `--help` for more details"),
        }

        InvalidReleaseFile (s: String) {
            description("Invalid release file"),
            display("Invalid release file, see `--help` for more details"),
        }

        CannotReadReleaseFile (path: PathBuf) {
            description("Cannot open release file"),
            display("Cannot open release file '{}'", path.display())
        }

        CannotWriteReleaseFile (path: PathBuf) {
            description("Cannot write release file"),
            display("Cannot write release file '{}'", path.display())
        }
    }
}

quick_main!(|| -> Result<()> {
    let app = App::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .author(crate_authors!("\n"))
        .arg(common::arg::file_path())
        .arg(common::arg::dry_run())
        .arg(common::arg::generate_autocompletion())
        .arg(common::arg::jcli())
        .subcommand(testnet::arg::command())
        .subcommand(release::arg::command());

    run_main(app)
});

fn run_main<'a, 'b>(mut app: App<'a, 'b>) -> Result<()> {
    let matches = app.clone().get_matches();

    if let Some(shell) = matches.value_of(common::arg::name::GENERATE_AUTOCOMPLETION) {
        // safe to unwrap as possible values have been validated first
        let shell = shell.parse().unwrap();

        app.gen_completions_to(crate_name!(), shell, &mut std::io::stdout());
        return Ok(());
    }

    let cfg = common::HelConfig::new(&matches)?;

    match matches.subcommand() {
        (testnet::arg::name::COMMAND, Some(matches)) => testnet::run(cfg, matches)?,
        (testnet::arg::name::COMMAND, None) => bail!(ErrorKind::ExpectedOptionsOrSubCommands(
            testnet::arg::name::COMMAND.to_owned()
        )),
        (release::arg::name::COMMAND, Some(matches)) => release::run(cfg, matches)?,
        (release::arg::name::COMMAND, None) => bail!(ErrorKind::ExpectedOptionsOrSubCommands(
            release::arg::name::COMMAND.to_owned()
        )),
        (cmd, _) => {
            if cmd.is_empty() {
                bail!(ErrorKind::NoCommand)
            }
            bail!(ErrorKind::UnknownCommand(cmd.to_owned()))
        }
    }

    Ok(())

    // hel release add v0.5.1 --channel v0.5 --channel v0.6 ...
    // * create a new release for jormungandr, query the github releases?
    // * check it's not already there and all
    //
    // hel release remove v0.5.1
    // * remove the release v0.5.1
    // * remove it from the list of entries that have this version
    //
    // hel testnet new <CHANNEL (v0.5) or (v0.5-nightly) (date added automatically)>
    // hel testnet dispose down <CHANNEL>
    // hel testnet remove <CHANNEL>
    //
}
