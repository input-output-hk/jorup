use crate::common::JorupConfig;
use std::path::{Path, PathBuf};

error_chain! {}

pub struct Channel {
    entry: jorup_lib::Entry,

    path: PathBuf,
}

impl Channel {
    pub fn new(cfg: &JorupConfig, entry: jorup_lib::Entry) -> Result<Self> {
        let path = cfg
            .channel_dir()
            .join(entry.channel().channel())
            .join(entry.channel().version().to_string());
        let path = if let Some(date) = entry.channel().nightly_date() {
            path.join(date)
        } else {
            path
        };
        std::fs::create_dir_all(&path)
            .chain_err(|| format!("Error while creating directory '{}'", path.display()))?;
        Ok(Channel { entry, path })
    }

    pub fn prepare(&self) -> Result<()> {
        self.install_genesis_file()?;
        self.install_block0()?;
        self.install_block0_hash()
    }

    fn install_genesis_file(&self) -> Result<()> {
        let path = self.get_genesis();
        let content = self.entry().genesis().content().as_bytes();

        write_all_to(&path, content).chain_err(|| format!("with file {}", path.display()))
    }

    fn install_block0(&self) -> Result<()> {
        let path = self.get_genesis_block();
        let content = hex::decode(self.entry().genesis().block0()).unwrap();

        write_all_to(&path, content).chain_err(|| format!("with file {}", path.display()))
    }

    fn install_block0_hash(&self) -> Result<()> {
        let path = self.get_genesis_block_hash();
        let content = self.entry().genesis().block0_hash();

        write_all_to(&path, content).chain_err(|| format!("with file {}", path.display()))
    }

    pub fn entry(&self) -> &jorup_lib::Entry {
        &self.entry
    }

    pub fn get_genesis(&self) -> PathBuf {
        self.dir().join("genesis.yaml")
    }

    pub fn get_genesis_block(&self) -> PathBuf {
        self.dir().join("genesis.block")
    }

    pub fn get_genesis_block_hash(&self) -> PathBuf {
        self.dir().join("genesis.block.hash")
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
