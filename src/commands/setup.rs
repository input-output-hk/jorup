use crate::common::JorupConfig;
use super::Cmd;
use std::{
    env::{self, consts::EXE_SUFFIX},
    fs, io,
    path::{Path, PathBuf},
};
use structopt::StructOpt;
use thiserror::Error;

/// Operations for 'jorup'
#[derive(Debug, StructOpt)]
pub enum Command {
    Install(Install),
    Update,
    Uninstall,
}

/// Install jorup
#[derive(Debug, StructOpt)]
pub struct Install {
    /// Don't change the local PATH variables
    #[structopt(long)]
    no_modify_path: bool,

    /// Even if a previous installed jorup is already installed, install
    /// this new version.
    #[structopt(short, long)]
    force: bool,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Common(#[from] crate::common::Error),
    #[error("jorup already installed")]
    AlreadyInstalled,
    #[error("Cannot get the current executable for the installer")]
    NoInstallerExecutable(#[source] io::Error),
    #[error("Cannot install jorup in {1}")]
    Install(#[source] io::Error, PathBuf),
    #[cfg(unix)]
    #[error("Cannot set permissions for {1}")]
    Permissions(#[source] io::Error, PathBuf),
    #[cfg(unix)]
    #[error("Cannot read file {1}")]
    Read(#[source] io::Error, PathBuf),
    #[cfg(unix)]
    #[error("Cannot write to file {1}")]
    Write(#[source] io::Error, PathBuf),
    #[cfg(windows)]
    #[error("Cannot update PATH in Windows registry")]
    WinregError(#[source] io::Error),
}

impl Command {
    pub fn run(self, cfg: JorupConfig) -> Result<(), Error> {
        match self {
            Command::Install(cmd) => cmd.run(cfg),
            Command::Update => update(cfg),
            Command::Uninstall => uninstall(cfg),
        }
    }
}

impl Install {
    pub fn run(self, cfg: JorupConfig) -> Result<(), Error> {
        let bin_dir = cfg.bin_dir();
        let jorup_file = bin_dir.join(format!("jorup{}", EXE_SUFFIX));

        if jorup_file.is_file() {
            let force = self.force
                || dialoguer::Confirmation::new()
                    .with_text("jorup is already installed, overwrite?")
                    .interact()
                    .unwrap();

            if !force {
                return Err(Error::AlreadyInstalled);
            }
        }

        let jorup_current = env::current_exe().map_err(Error::NoInstallerExecutable)?;
        fs::copy(&jorup_current, &jorup_file).map_err(|e| Error::Install(e, jorup_file.clone()))?;
        make_executable(&jorup_file)?;

        if !self.no_modify_path {
            do_add_to_path(&cfg)?;
        }

        Ok(())
    }
}

impl Cmd for Install {
    type Err = Error;

    fn run(self) -> Result<(), Self::Err> {
        let cfg = crate::common::JorupConfig::new(None, None, false)?;
        self.run(cfg)
    }
}

pub fn uninstall(_cfg: JorupConfig) -> Result<(), Error> {
    unimplemented!()
}

pub fn update(_cfg: JorupConfig) -> Result<(), Error> {
    unimplemented!()
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::metadata(path).map_err(|e| Error::Permissions(e, path.to_path_buf()))?;
    let mut perms = metadata.permissions();
    let mode = perms.mode();
    let new_mode = (mode & !0o777) | 0o755;

    if mode == new_mode {
        return Ok(());
    }

    perms.set_mode(new_mode);
    fs::set_permissions(path, perms).map_err(|e| Error::Permissions(e, path.to_path_buf()))
}

#[cfg(windows)]
fn make_executable(_: &Path) -> Result<(), Error> {
    Ok(())
}

#[cfg(unix)]
fn do_add_to_path(cfg: &JorupConfig) -> Result<(), Error> {
    let methods = get_add_path_methods();

    for rcpath in methods {
        let file = if rcpath.exists() {
            fs::read_to_string(&rcpath).map_err(|e| Error::Read(e, rcpath.clone()))?
        } else {
            String::new()
        };
        let addition = format!("\n{}", shell_export_string(cfg)?);
        if !file.contains(&addition) {
            use std::io::Write as _;
            let mut writer = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&rcpath)
                .map_err(|e| Error::Write(e, rcpath.clone()))?;
            writer
                .write_all(addition.as_bytes())
                .map_err(|e| Error::Write(e, rcpath.clone()))?;
        }
    }

    Ok(())
}

#[cfg(windows)]
fn do_add_to_path(cfg: &JorupConfig) -> Result<(), Error> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let environment = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .map_err(Error::WinregError)?;

    let current_path: String = environment.get_value("Path").map_err(Error::WinregError)?;
    let jorup_path = cfg.bin_dir().display().to_string();

    if current_path.contains(&jorup_path) {
        return Ok(());
    }

    let new_path = format!("{};{}", jorup_path, current_path);
    environment
        .set_value("Path", &new_path)
        .map_err(Error::WinregError)?;

    Ok(())
}

/// Decide which rcfiles we're going to update, so we can tell the user before
/// they confirm.
#[cfg(unix)]
fn get_add_path_methods() -> Vec<PathBuf> {
    let profile = dirs::home_dir().map(|p| p.join(".profile"));
    let mut profiles = vec![profile];

    if let Ok(shell) = env::var("SHELL") {
        if shell.contains("zsh") {
            let zdotdir = env::var("ZDOTDIR")
                .ok()
                .map(PathBuf::from)
                .or_else(dirs::home_dir);
            let zprofile = zdotdir.map(|p| p.join(".zprofile"));
            profiles.push(zprofile);
        }
    }

    if let Some(bash_profile) = dirs::home_dir().map(|p| p.join(".bash_profile")) {
        // Only update .bash_profile if it exists because creating .bash_profile
        // will cause .profile to not be read
        if bash_profile.exists() {
            profiles.push(Some(bash_profile));
        }
    }

    let rcfiles = profiles.into_iter().filter_map(|f| f);
    rcfiles.collect()
}

#[cfg(unix)]
fn shell_export_string(cfg: &JorupConfig) -> Result<String, Error> {
    let path = cfg.bin_dir().display().to_string();
    // The path is *pre-pended* in case there are system-installed
    Ok(format!(r#"export PATH="{}:$PATH""#, path))
}
