use crate::common::JorupConfig;
use crate::jorfile::PartialChannelDesc;
use semver::VersionReq;
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

const CHANNEL_NAME: &str = "CHANNEL_NAME";

pub struct Channel {
    entry: crate::jorfile::Entry,
    version: PartialChannelDesc,

    path: PathBuf,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("No jorfile... Cannot operate")]
    NoJorfile(#[source] crate::common::Error),
    #[error("No entry available for the given version")]
    NoEntry,
    #[error("Cannot create directory: {1}")]
    CannotCreateDirectory(#[source] io::Error, PathBuf),
    #[error("Cannot write to file: {1}")]
    CannotWriteFile(#[source] io::Error, PathBuf),
}

impl Channel {
    pub fn arg<'a, 'b>() -> clap::Arg<'a, 'b>
    where
        'a: 'b,
    {
        clap::Arg::with_name(CHANNEL_NAME)
            .value_name("CHANNEL")
            .help("The channel to run jormungandr for, jorup uses the default channel otherwise")
            .validator(|s: String| {
                s.parse::<PartialChannelDesc>()
                    .map(|_channel| ())
                    .map_err(|err| format!("{}", err))
            })
    }

    pub fn load<'a, 'b>(
        cfg: &'b mut JorupConfig,
        args: &clap::ArgMatches<'a>,
    ) -> Result<Self, Error> {
        let mut channel_entered = cfg.current_channel().clone();

        let entry = if let Some(channel) = args.value_of(CHANNEL_NAME) {
            let jor = cfg.load_jor().map_err(Error::NoJorfile)?;

            // should be save to unwrap as we have set a validator in the Argument
            // for the CLI to check it is valid
            channel_entered = channel.parse().unwrap();

            jor.entries()
                .values()
                .filter(|entry| channel_entered.matches(entry.channel()))
                .last()
                .cloned()
        } else {
            cfg.current_entry().map_err(Error::NoJorfile)?.cloned()
        };

        if let Some(entry) = entry {
            Self::new(cfg, entry.clone(), channel_entered)
        } else {
            Err(Error::NoEntry)
        }
    }

    fn new(
        cfg: &JorupConfig,
        entry: crate::jorfile::Entry,
        channel_version: PartialChannelDesc,
    ) -> Result<Self, Error> {
        let path = cfg
            .channel_dir()
            .join(entry.channel().channel().to_string())
            .join(entry.channel().date().to_string());
        std::fs::create_dir_all(&path)
            .map_err(|e| Error::CannotCreateDirectory(e, path.clone()))?;
        Ok(Channel {
            entry,
            version: channel_version,
            path,
        })
    }

    pub fn channel_version(&self) -> &PartialChannelDesc {
        &self.version
    }

    pub fn prepare(&self) -> Result<(), Error> {
        self.install_block0_hash()
    }

    fn install_block0_hash(&self) -> Result<(), Error> {
        let path = self.get_genesis_block_hash();
        let content = self.entry().genesis().block0_hash();

        write_all_to(&path, content).map_err(|e| Error::CannotWriteFile(e, path))
    }

    pub fn jormungandr_version_req(&self) -> &VersionReq {
        self.entry().jormungandr_versions()
    }

    pub fn entry(&self) -> &crate::jorfile::Entry {
        &self.entry
    }

    pub fn get_log_file(&self) -> PathBuf {
        self.dir().join("NODE.logs")
    }

    pub fn get_runner_file(&self) -> PathBuf {
        self.dir().join("running_config.toml")
    }

    pub fn get_genesis_block_hash(&self) -> PathBuf {
        self.dir().join("genesis.block.hash")
    }

    pub fn get_node_storage(&self) -> PathBuf {
        self.dir().join("node-storage")
    }

    pub fn get_node_config(&self) -> PathBuf {
        self.dir().join("node-config.yaml")
    }

    pub fn get_node_secret(&self) -> PathBuf {
        self.dir().join("node-secret.yaml")
    }

    pub fn get_wallet_secret(&self) -> PathBuf {
        self.dir().join("wallet.secret.key")
    }

    pub fn dir(&self) -> &PathBuf {
        &self.path
    }
}

fn write_all_to<P, C>(path: P, content: C) -> std::io::Result<()>
where
    P: AsRef<Path>,
    C: AsRef<[u8]>,
{
    if path.as_ref().is_file() {
        return Ok(());
    }

    std::fs::write(path, content)
}
