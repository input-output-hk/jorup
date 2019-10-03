use crate::{Error, ErrorKind, Result, ResultExt};
use clap::ArgMatches;
use jorup_lib::Jor;
use semver::{Version, VersionReq};
use std::{ffi::OsString, path::PathBuf, process::Command};

#[derive(Debug)]
pub struct HelConfig {
    file_path: PathBuf,
    dry_run: bool,
    jcli: OsString,
}

impl HelConfig {
    pub fn new<'a>(args: &ArgMatches<'a>) -> Result<Self> {
        let file = if let Some(file) = args.value_of(arg::name::FILE_PATH) {
            file.parse()
                .chain_err(|| ErrorKind::InvalidReleaseFile(file.to_owned()))?
        } else {
            bail!(ErrorKind::MissingReleaseFile)
        };

        Ok(HelConfig {
            dry_run: args.is_present(arg::name::DRY_RUN),
            jcli: args.value_of_os(arg::name::JCLI).unwrap().to_owned(),
            file_path: file,
        })
    }

    pub fn jcli(&self, version_req: &VersionReq) -> Result<Command> {
        let version = Command::new(&self.jcli).arg("--version").output()?;
        let version = String::from_utf8(version.stdout).unwrap();

        let version: Version = version.trim_start_matches("jcli ").parse().unwrap();

        if version_req.matches(&version) {
            Ok(Command::new(&self.jcli))
        } else {
            bail!("Invalid jcli version")
        }
    }

    pub fn save_release_file(&self, jor: Jor) -> Result<()> {
        if self.dry_run {
            return Ok(());
        }

        let file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&self.file_path)?;
        serde_json::to_writer(file, &jor)
            .chain_err(|| ErrorKind::CannotWriteReleaseFile(self.file_path.clone()))
    }

    pub fn load_release_file(&self) -> Result<Jor> {
        use std::fs::File;
        match File::open(&self.file_path) {
            Ok(mut file) => serde_json::from_reader(&mut file)
                .chain_err(|| ErrorKind::CannotReadReleaseFile(self.file_path.clone())),
            Err(err) => {
                use std::io::ErrorKind::*;
                match err.kind() {
                    NotFound => Ok(Jor::default()),
                    _ => bail!(Error::with_chain(
                        err,
                        ErrorKind::CannotReadReleaseFile(self.file_path.clone())
                    )),
                }
            }
        }
    }
}

pub mod arg {
    use clap::Arg;

    pub mod name {
        pub const FILE_PATH: &str = "HEL_PATH";
        pub const DRY_RUN: &str = "DRY_RUN";
        pub const JCLI: &str = "JCLI";
        pub const GENERATE_AUTOCOMPLETION: &str = "GENERATE_AUTOCOMPLETION";
    }

    pub fn file_path<'a, 'b>() -> Arg<'a, 'b> {
        Arg::with_name(name::FILE_PATH)
            .long("file")
            .alias("hel-file")
            .help("set the file where the releases are stored and/or will be added or updated")
            .long_help(
                "This is the file path of the releases and testnets data. It must be a valid
file path to a readable and writable file for the current user. Note that this
option will not update the release file if the option `--dry-run` has been set.
Equally, this value can be set via the environment variable `HEL_FILE`",
            )
            .takes_value(true)
            .value_name("HEL_FILE")
            .env("HEL_FILE")
            .multiple(false)
            .global(true)
    }

    pub fn jcli<'a, 'b>() -> Arg<'a, 'b> {
        Arg::with_name(name::JCLI)
            .long("jcli")
            .help("specify path to jormungandr command line interface.")
            .long_help(
                "Hel needs `jcli` to operate on some validation and operations. This
will specify what version of jcli to utilize for that process.
Equally, this value can be set via the environment variable `JCLI`. By default
we will utilise the one specified in the $PATH.",
            )
            .takes_value(true)
            .value_name("JCLI")
            .default_value("jcli")
            .env("JCLI")
            .multiple(false)
            .global(true)
    }

    pub fn generate_autocompletion<'a, 'b>() -> Arg<'a, 'b> {
        Arg::with_name(name::GENERATE_AUTOCOMPLETION)
            .long("generate-auto-completion")
            .help("generate autocompletion scripts for the given <SHELL>")
            .long_help(
                "Generate the autocompletion scripts for the given shell,
Autocompletion will be written in the standard output and can then be pasted
by the user to the appropriate place",
            )
            .takes_value(true)
            .possible_values(&clap::Shell::variants())
            .value_name("SHELL")
            .multiple(false)
            .global(true)
    }

    pub fn dry_run<'a, 'b>() -> Arg<'a, 'b> {
        Arg::with_name(name::DRY_RUN)
            .long("dry-run")
            .help("Don't apply the operations in the release file (`--file`)")
            .long_help(
                "Dry run the operation. Executing all the commands but without saving the
results. This allows to test a command before applying it to the final file",
            )
            .hidden_short_help(true)
            .global(true)
    }
}
