use crate::utils::version::VersionReq;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Config(Vec<Blockchain>);

#[derive(Debug, Clone, Deserialize)]
pub struct Blockchain {
    name: String,
    description: String,
    jormungandr_versions: VersionReq,
    block0_hash: String,
    trusted_peers: Vec<TrustedPeer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedPeer {
    address: String,
}

impl Config {
    pub fn get_blockchain(&self, name: &str) -> Option<&Blockchain> {
        self.0.iter().find(|blockchain| blockchain.name() == name)
    }

    pub fn blockchains(&self) -> &[Blockchain] {
        &self.0
    }
}

impl Blockchain {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn jormungandr_versions(&self) -> &VersionReq {
        &self.jormungandr_versions
    }

    pub fn block0_hash(&self) -> &str {
        &self.block0_hash
    }

    pub fn trusted_peers(&self) -> &[TrustedPeer] {
        &self.trusted_peers
    }
}

impl TrustedPeer {
    pub fn address(&self) -> &str {
        &self.address
    }
}
