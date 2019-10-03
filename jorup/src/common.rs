use clap::ArgMatches;
use std::path::PathBuf;

#[derive(Debug)]
pub struct JorupConfig {
    home_dir: PathBuf,
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

        let cfg = JorupConfig { home_dir };

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

    pub fn bin_dir(&self) -> PathBuf {
        self.home_dir.join("bin")
    }
    pub fn channel_dir(&self) -> PathBuf {
        self.home_dir.join("channel")
    }
    pub fn release_dir(&self) -> PathBuf {
        self.home_dir.join("release")
    }
}

pub mod arg {
    use super::Result;
    use clap::Arg;

    pub mod name {
        pub const GENERATE_AUTOCOMPLETION: &str = "GENERATE_AUTOCOMPLETION";
        pub const JORUP_HOME: &str = "JORUP_HOME";
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
