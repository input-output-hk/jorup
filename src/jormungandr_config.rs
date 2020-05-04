use serde::{Deserialize, Serialize};
use std::{path::PathBuf, net::SocketAddr};

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub log: Vec<Log>,
    pub p2p: P2p,
    pub rest: Rest,
    pub storage: PathBuf,
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
    pub public_address: String,
    pub trusted_peers: Vec<crate::config::TrustedPeer>,
}

#[derive(Deserialize, Serialize)]
pub struct Rest {
    pub listen: SocketAddr,
}
