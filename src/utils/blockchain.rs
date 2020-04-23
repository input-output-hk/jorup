use crate::{common::JorupConfig, utils::version::VersionReq};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

pub struct Blockchain {
    entry: crate::config::Blockchain,

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

impl Blockchain {
    pub fn load(cfg: &mut JorupConfig, blockchain_name: &str) -> Result<Self, Error> {
        let jor = cfg.load_jor().map_err(Error::NoJorfile)?;

        let entry = jor.get_blockchain(blockchain_name).cloned();

        if let Some(entry) = entry {
            Self::new(cfg, entry.clone())
        } else {
            Err(Error::NoEntry)
        }
    }

    fn new(cfg: &JorupConfig, entry: crate::config::Blockchain) -> Result<Self, Error> {
        let path = cfg.blockchain_dir().join(entry.name().to_string());
        std::fs::create_dir_all(&path)
            .map_err(|e| Error::CannotCreateDirectory(e, path.clone()))?;
        Ok(Self { entry, path })
    }

    pub fn prepare(&self) -> Result<(), Error> {
        self.install_block0_hash()
    }

    fn install_block0_hash(&self) -> Result<(), Error> {
        let path = self.get_genesis_block_hash();
        let content = self.entry().block0_hash();

        write_all_to(&path, content).map_err(|e| Error::CannotWriteFile(e, path))
    }

    pub fn jormungandr_version_req(&self) -> &VersionReq {
        self.entry().jormungandr_versions()
    }

    pub fn entry(&self) -> &crate::config::Blockchain {
        &self.entry
    }

    pub fn get_log_file(&self) -> PathBuf {
        self.dir().join("NODE.logs")
    }

    pub fn get_runner_file(&self) -> PathBuf {
        self.dir().join("running_config.json")
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
