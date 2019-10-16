use crate::common::JorupConfig;
use clap::ArgMatches;
use std::{
    env::{self, consts::EXE_SUFFIX},
    fs,
    path::{Path, PathBuf},
};

error_chain! {}

pub mod arg {
    use clap::{App, Arg, SubCommand};

    pub mod name {
        pub const COMMAND: &str = "self";
    }

    pub fn commands<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name(name::COMMAND)
            .about("operations for 'jorup'")
            .subcommand(
                SubCommand::with_name("install").arg(
                    Arg::with_name("NO_MODIFY_PATH")
                        .long("no-modify-path")
                        .help("Don't change the local PATH variables"),
                )
                .arg(
                    Arg::with_name("FORCE_INSTALL")
                        .long("force")
                        .short("f")
                        .help("Even if a previous installed jorup is already installed, install this new version")
                )
            )
            .subcommand(SubCommand::with_name("update"))
            .subcommand(SubCommand::with_name("uninstall").alias("remove"))
    }
}

pub fn run<'a>(cfg: JorupConfig, args: &ArgMatches<'a>) -> Result<()> {
    match args.subcommand() {
        ("update", matches) => update(cfg, matches),
        ("uninstall", matches) => uninstall(cfg, matches),
        ("install", matches) => install(cfg, matches),
        (cmd, _) => {
            if cmd.is_empty() {
                bail!("No command given")
            }
            bail!(format!("Unknown command {}", cmd))
        }
    }
}

pub fn install<'a>(cfg: JorupConfig, args: Option<&ArgMatches<'a>>) -> Result<()> {
    let no_modify_path = args
        .map(|args| args.is_present("NO_MODIFY_PATH"))
        .unwrap_or(false);
    let force = args
        .map(|args| args.is_present("FORCE_INSTALL"))
        .unwrap_or(false);

    let bin_dir = cfg.bin_dir();
    let jorup_file = bin_dir.join(format!("jorup{}", EXE_SUFFIX));

    if jorup_file.is_file() {
        let force = force
            || dialoguer::Confirmation::new()
                .with_text("jorup is already installed, overwrite?")
                .interact()
                .unwrap();

        if !force {
            bail!(format!("jorup already installed: {}", jorup_file.display()))
        }
    }

    let jorup_current = std::env::current_exe()
        .chain_err(|| "Cannot get the current executable for the installer")?;
    std::fs::copy(&jorup_current, &jorup_file)
        .chain_err(|| format!("Cannot install jorup in {}", jorup_file.display()))?;
    make_executable(&jorup_file).chain_err(|| "Cannot make installed bin executable")?;

    if !no_modify_path {
        do_add_to_path(&cfg, &get_add_path_methods()).chain_err(|| "Cannot update the PATH")?;
    }

    Ok(())
}

pub fn uninstall<'a>(cfg: JorupConfig, args: Option<&ArgMatches<'a>>) -> Result<()> {
    unimplemented!()
}

pub fn update<'a>(cfg: JorupConfig, args: Option<&ArgMatches<'a>>) -> Result<()> {
    unimplemented!()
}

pub fn make_executable(path: &Path) -> Result<()> {
    #[cfg(windows)]
    fn inner(_: &Path) -> Result<()> {
        Ok(())
    }
    #[cfg(not(windows))]
    fn inner(path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let metadata = fs::metadata(path)
            .chain_err(|| format!("Cannot set permission to {}", path.display()))?;
        let mut perms = metadata.permissions();
        let mode = perms.mode();
        let new_mode = (mode & !0o777) | 0o755;

        if mode == new_mode {
            return Ok(());
        }

        perms.set_mode(new_mode);
        set_permissions(path, perms)
    }

    inner(path)
}

pub fn set_permissions(path: &Path, perms: fs::Permissions) -> Result<()> {
    fs::set_permissions(path, perms)
        .chain_err(|| format!("Cannot set permissions to {}", path.display()))
}

#[derive(PartialEq)]
enum PathUpdateMethod {
    RcFile(PathBuf),
    Windows,
}

/// Decide which rcfiles we're going to update, so we
/// can tell the user before they confirm.
fn get_add_path_methods() -> Vec<PathUpdateMethod> {
    if cfg!(windows) {
        return vec![PathUpdateMethod::Windows];
    }

    let profile = env::home_dir().map(|p| p.join(".profile"));
    let mut profiles = vec![profile];

    if let Ok(shell) = env::var("SHELL") {
        if shell.contains("zsh") {
            let zdotdir = env::var("ZDOTDIR")
                .ok()
                .map(PathBuf::from)
                .or_else(env::home_dir);
            let zprofile = zdotdir.map(|p| p.join(".zprofile"));
            profiles.push(zprofile);
        }
    }

    if let Some(bash_profile) = env::home_dir().map(|p| p.join(".bash_profile")) {
        // Only update .bash_profile if it exists because creating .bash_profile
        // will cause .profile to not be read
        if bash_profile.exists() {
            profiles.push(Some(bash_profile));
        }
    }

    let rcfiles = profiles.into_iter().filter_map(|f| f);
    rcfiles.map(PathUpdateMethod::RcFile).collect()
}

fn shell_export_string(cfg: &JorupConfig) -> Result<String> {
    let path = cfg.bin_dir().display().to_string();
    // The path is *pre-pended* in case there are system-installed
    Ok(format!(r#"export PATH="{}:$PATH""#, path))
}

#[cfg(unix)]
fn do_add_to_path(cfg: &JorupConfig, methods: &[PathUpdateMethod]) -> Result<()> {
    for method in methods {
        if let PathUpdateMethod::RcFile(ref rcpath) = *method {
            let file = if rcpath.exists() {
                fs::read_to_string(rcpath)
                    .chain_err(|| format!("Cannot read {}", rcpath.display()))?
            } else {
                String::new()
            };
            let addition = format!("\n{}", shell_export_string(cfg)?);
            if !file.contains(&addition) {
                use std::io::Write as _;
                let mut writer = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(rcpath)
                    .chain_err(|| {
                        format!(
                            "Cannot open file to write the Path in: {}",
                            rcpath.display()
                        )
                    })?;
                writer
                    .write_all(addition.as_bytes())
                    .chain_err(|| format!("Cannot append PATH in {}", rcpath.display()))?;
            }
        } else {
            unreachable!()
        }
    }

    Ok(())
}

#[cfg(windows)]
fn do_add_to_path(_cfg: &JorupConfig, _methods: &[PathUpdateMethod]) -> Result<()> {
    bail!("Windows support not fully implemented yet")
}
