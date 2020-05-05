use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    net::SocketAddr,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub log: Vec<Log>,
    pub p2p: Option<P2p>,
    pub rest: Option<Rest>,
    pub storage: Option<PathBuf>,
    #[serde(default)]
    pub secret_files: Vec<PathBuf>,
}

#[derive(Deserialize, Serialize)]
pub struct Log {
    pub output: String,
    pub level: String,
    pub format: String,
}

#[derive(Deserialize, Serialize)]
pub struct P2p {
    pub public_address: Option<String>,
    #[serde(default)]
    pub trusted_peers: Vec<crate::config::TrustedPeer>,
}

#[derive(Deserialize, Serialize)]
pub struct Rest {
    pub listen: SocketAddr,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("the configuration file should be either yaml or json")]
    UnknownFileFormat,
    #[error("failed to open file")]
    Io(#[from] std::io::Error),
    #[error("failed to read JSON configuration file")]
    Json(#[from] serde_json::Error),
    #[error("failed to read YAML configuration file")]
    Yaml(#[from] serde_yaml::Error),
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, Error> {
    let path = path.as_ref();

    match path.extension().map(|os_str| os_str.to_str()).flatten() {
        Some("json") => serde_json::from_reader(File::open(path)?).map_err(Into::into),
        Some("yaml") | Some("yml") => {
            serde_yaml::from_reader(File::open(path)?).map_err(Into::into)
        }
        _ => Err(Error::UnknownFileFormat),
    }
}
