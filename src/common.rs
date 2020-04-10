use crate::jorfile::PartialChannelDesc;
use crate::utils::download_file;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, io, path::PathBuf};
use thiserror::Error;

#[derive(Debug)]
pub struct JorupConfig {
    home_dir: PathBuf,
    settings: JorupSettings,

    jor_file: Option<PathBuf>,
    jor: Option<crate::jorfile::Jor>,
    offline: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JorupSettings {
    default: PartialChannelDesc,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("No $HOME environment variable, can not set JORUP_HOME value.")]
    NoHomeDir,
    #[error("Cannot create JORUP_HOME: {1}")]
    CannotCreateHomeDir(#[source] io::Error, PathBuf),
    #[error("Cannot create directory: {1}")]
    CannotCreateInitDir(#[source] io::Error, PathBuf),
    #[error("Cannot save settings: {1}")]
    CannotSaveSettings(#[source] io::Error, PathBuf),
    #[error("Cannot open file: {1}")]
    CannotOpenFile(#[source] io::Error, PathBuf),
    #[error("Cannot parse file: {1}")]
    Json(#[source] serde_json::Error, PathBuf),
    #[error("Cannot sync jorfile with registry")]
    CannotSyncRegistry(#[source] crate::utils::download::Error),
}

impl JorupConfig {
    pub fn new(
        jorup_home: Option<PathBuf>,
        jorfile: Option<PathBuf>,
        offline: bool,
    ) -> Result<Self, Error> {
        let home_dir = jorup_home
            .or_else(|| dirs::home_dir().map(|d| d.join(".jorup")))
            .ok_or_else(|| Error::NoHomeDir)?;

        let home_dir = if home_dir.is_absolute() {
            home_dir
        } else {
            std::env::current_dir().unwrap().join(home_dir)
        };

        std::fs::create_dir_all(&home_dir)
            .map_err(|e| Error::CannotCreateHomeDir(e, home_dir.clone()))?;

        let jor_file = jorfile.map(|jor_file| jor_file.into());

        let mut cfg = JorupConfig {
            home_dir,
            settings: JorupSettings::default(),
            jor_file,
            jor: None,
            offline,
        };

        cfg.init()?;
        cfg.load_settings()?;
        cfg.detect_installed_path();

        Ok(cfg)
    }

    fn init(&self) -> Result<(), Error> {
        std::fs::create_dir_all(self.bin_dir())
            .map_err(|e| Error::CannotCreateInitDir(e, self.bin_dir()))?;
        std::fs::create_dir_all(self.channel_dir())
            .map_err(|e| Error::CannotCreateInitDir(e, self.channel_dir()))?;
        std::fs::create_dir_all(self.release_dir())
            .map_err(|e| Error::CannotCreateInitDir(e, self.release_dir()))?;

        if !self.jorup_settings_file().is_file() {
            self.save_settings()?;
        }

        Ok(())
    }

    fn detect_installed_path(&self) {
        let bin_dir = if self.bin_dir().is_absolute() {
            self.bin_dir()
        } else {
            std::env::current_dir().unwrap().join(self.bin_dir())
        };
        match std::env::var_os("PATH") {
            Some(paths) => {
                let present = std::env::split_paths(&paths).any(|path| path == bin_dir);
                if !present {
                    eprintln!(
                        "WARN: environment PATH does not contain bin dir: {}",
                        bin_dir.display()
                    );
                }

                let others: BTreeSet<_> = std::env::split_paths(&paths)
                    .filter(|path| path != &bin_dir)
                    .filter(|path| path.join("jormungandr").is_file())
                    .collect();
                for other in others {
                    eprintln!("WARN: found competing installation in {}", other.display());
                }
            }
            None => {
                eprintln!("WARN: no environment PATH recognized on this system");
            }
        }
    }

    pub fn settings(&self) -> &JorupSettings {
        &self.settings
    }

    pub fn current_channel(&self) -> &PartialChannelDesc {
        &self.settings().default
    }

    pub fn current_entry(&mut self) -> Result<Option<&crate::jorfile::Entry>, Error> {
        self.load_jor()?;
        let current_default = &self.settings.default;

        Ok(self
            .jor
            .as_ref()
            .unwrap()
            .entries()
            .values()
            .filter(|entry| current_default.matches(entry.channel()))
            .last())
    }

    pub fn set_default_channel(&mut self, new_default: PartialChannelDesc) -> Result<(), Error> {
        self.settings.default = new_default;
        self.save_settings()
    }

    fn load_settings(&mut self) -> Result<(), Error> {
        let json = std::fs::read_to_string(self.jorup_settings_file())
            .map_err(|e| Error::CannotOpenFile(e, self.jorup_settings_file()))?;

        self.settings =
            serde_json::from_str(&json).map_err(|e| Error::Json(e, self.jorup_settings_file()))?;
        Ok(())
    }

    fn save_settings(&self) -> Result<(), Error> {
        std::fs::write(
            self.jorup_settings_file(),
            serde_json::to_vec(&self.settings)
                .map_err(|e| Error::Json(e, self.jorup_settings_file()))?,
        )
        .map_err(|e| Error::CannotSaveSettings(e, self.jorup_settings_file()))
    }

    pub fn jorfile(&self) -> PathBuf {
        self.jor_file
            .clone()
            .unwrap_or_else(|| self.home_dir.join("jorfile.json"))
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
    pub fn jorup_settings_file(&self) -> PathBuf {
        self.home_dir.join("settings.json")
    }

    pub fn offline(&self) -> bool {
        self.offline
    }

    pub fn sync_jorfile(&self) -> Result<(), Error> {
        // do not sync if the jorfile was given as parameter of the
        // command line or if `--offline`
        if self.jor_file.is_some() || self.offline {
            return Ok(());
        }

        download_file(
            "jorfile",
            "https://raw.githubusercontent.com/input-output-hk/jorup/master/jorfile.json",
            self.jorfile(),
        )
        .map_err(Error::CannotSyncRegistry)
    }

    pub fn load_jor(&mut self) -> Result<&crate::jorfile::Jor, Error> {
        if self.jor.is_none() {
            let file = std::fs::File::open(self.jorfile())
                .map_err(|e| Error::CannotOpenFile(e, self.jorfile()))?;

            let jor = serde_json::from_reader(file).map_err(|e| Error::Json(e, self.jorfile()))?;
            self.jor = Some(jor);
        }

        Ok(self.jor.as_ref().unwrap())
    }
}

impl Default for JorupSettings {
    fn default() -> Self {
        JorupSettings {
            default: PartialChannelDesc::default(),
        }
    }
}
