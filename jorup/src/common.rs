use clap::ArgMatches;
use std::path::PathBuf;

#[derive(Debug)]
pub struct JorupConfig {
    home_dir: PathBuf,

    jor_file: Option<PathBuf>,
    offline: bool,
}

error_chain! {
    errors {
        NoHOMEDir {
            description("No $HOME environment variable, can not set JORUP_HOME value.")
        }
        CannotCreateHomeDir(home_dir: PathBuf) {
            description("Cannot create the JORUP_HOME directory"),
            display("Cannot create JORUP_HOME [={}]", home_dir.display()),
        }
        CannotCreateInitDir(init_dir: PathBuf) {
            description("Cannot create one of the main HOME directory"),
            display("Cannot create directory [={}]", init_dir.display()),
        }
    }
}

impl JorupConfig {
    pub fn new<'a>(args: &ArgMatches<'a>) -> Result<Self> {
        let home_dir = value_t!(args, arg::name::JORUP_HOME, PathBuf).unwrap();
        std::fs::create_dir_all(&home_dir)
            .chain_err(|| ErrorKind::CannotCreateHomeDir(home_dir.clone()))?;

        let jor_file = if let Some(jor_file) = args.value_of(arg::name::JOR_FILE) {
            Some(jor_file.into())
        } else {
            None
        };
        let cfg = JorupConfig {
            home_dir,
            jor_file,
            offline: args.is_present(arg::name::OFFLINE),
        };

        cfg.init()?;

        Ok(cfg)
    }

    fn init(&self) -> Result<()> {
        std::fs::create_dir_all(self.bin_dir())
            .chain_err(|| ErrorKind::CannotCreateInitDir(self.bin_dir()))?;
        std::fs::create_dir_all(self.channel_dir())
            .chain_err(|| ErrorKind::CannotCreateInitDir(self.channel_dir()))?;
        std::fs::create_dir_all(self.release_dir())
            .chain_err(|| ErrorKind::CannotCreateInitDir(self.release_dir()))?;
        Ok(())
    }

    pub fn jorfile(&self) -> PathBuf {
        self.jor_file
            .clone()
            .unwrap_or(self.home_dir.join("jorfile.json"))
    }
    pub fn bin_dir(&self) -> PathBuf {
        self.home_dir.join("bin")
    }
    pub fn channel_dir(&self) -> PathBuf {
        self.home_dir.join("channel")
    }
    pub fn release_dir(&self) -> PathBuf {
        self.home_dir.join("release")
    }

    pub fn offline(&self) -> bool {
        self.offline
    }

    pub fn sync_jorfile(&self) -> Result<()> {
        // do not sync if the jorfile was given as parameter of the
        // command line or if `--offline`
        if self.jor_file.is_some() || self.offline {
            return Ok(());
        }

        unimplemented!("fetching jor file from the network is not supported yet")
    }

    pub fn load_jor(&self) -> Result<jorup_lib::Jor> {
        let file = std::fs::File::open(self.jorfile())
            .chain_err(|| format!("Cannot open file {}", self.jorfile().display()))?;

        serde_json::from_reader(file)
            .chain_err(|| format!("cannot parse file {}", self.jorfile().display()))
    }
}

pub mod arg {
    use super::Result;
    use clap::Arg;

    pub mod name {
        pub const GENERATE_AUTOCOMPLETION: &str = "GENERATE_AUTOCOMPLETION";
        pub const JORUP_HOME: &str = "JORUP_HOME";
        pub const JOR_FILE: &str = "JOR_FILE";
        pub const OFFLINE: &str = "JORUP_OFFLINE";
    }

    pub fn jorup_home<'a, 'b>() -> Result<Arg<'a, 'b>> {
        let arg = Arg::with_name(name::JORUP_HOME)
            .long("jorup-home")
            .help("Set the directory home for jorup")
            .long_help(
                "Set the directory path where jorup will install the different
releases or different channels. Mainly remember to set `$JORUP_HOME/bin` value to your
$PATH for easy access to the default release's tools
",
            )
            .takes_value(true)
            .env(name::JORUP_HOME)
            .value_name(name::JORUP_HOME)
            .default_value_os(super::JORUP_HOME.as_os_str())
            .multiple(false)
            .global(true);
        Ok(arg)
    }

    pub fn jor_file<'a, 'b>() -> Arg<'a, 'b> {
        Arg::with_name(name::JOR_FILE)
            .long("jorfile")
            .help("don't use the jor file from from local setting but use given one")
            .long_help(
                "This is not to be used lightly as it may put your local jor in an invalid
state. Instead of fetching the jorfile from the network and/or to use the local one, use
a specific file. This is useful only for testing. This option does not imply offline.",
            )
            .takes_value(true)
            .value_name(name::JOR_FILE)
            .multiple(false)
            .hidden_short_help(true)
            .global(true)
    }

    pub fn offline<'a, 'b>() -> Arg<'a, 'b> {
        Arg::with_name(name::OFFLINE)
            .long("offline")
            .help("don't query the release server to update the index")
            .long_help(
                "Try only to work with the current states and values. Do not attempt to
update the known releases and testnets. This may make your system to fail to install specific
releases if they are not already cached locally.",
            )
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
}

lazy_static! {
    static ref JORUP_HOME: PathBuf = { jorup_home().unwrap() };
}

fn jorup_home() -> Result<PathBuf> {
    home::home_dir()
        .map(|d| d.join(".jorup"))
        .ok_or_else(|| ErrorKind::NoHOMEDir.into())
}
