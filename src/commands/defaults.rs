use crate::{common::JorupConfig, utils::blockchain::Blockchain};
use structopt::StructOpt;
use thiserror::Error;

/// Output the default configuration for the given blockchain. This
/// configuration can be customized and provided to `jorup run` later.
#[derive(Debug, StructOpt)]
pub struct Command {
    /// The blockchain to get the configuration for
    blockchain: String,

    #[structopt(long, default_value = "yaml")]
    format: ConfigFormat,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot run the node without valid blockchain")]
    NoValidBlockchain(#[source] crate::utils::blockchain::Error),
    #[error("Could not write JSON")]
    Json(#[source] serde_json::Error),
    #[error("Could not write YAML")]
    Yaml(#[source] serde_yaml::Error),
}

#[derive(Debug)]
enum ConfigFormat {
    Json,
    Yaml,
}

mod config {
    use serde::Serialize;
    use std::path::PathBuf;

    #[derive(Serialize)]
    pub struct Config {
        pub log: Vec<Log>,
        pub p2p: P2p,
        pub rest: Rest,
        pub storage: PathBuf,
        pub secret_files: Vec<PathBuf>,
    }

    #[derive(Serialize)]
    pub struct Log {
        pub output: String,
        pub level: String,
        pub format: String,
    }

    #[derive(Serialize)]
    pub struct P2p {
        pub public_address: String,
        pub trusted_peers: Vec<crate::config::TrustedPeer>,
    }

    #[derive(Serialize)]
    pub struct Rest {
        pub listen: String,
    }
}

#[derive(Debug, Error)]
#[error("Unknown configuration format value")]
struct ConfigFormatError;

impl Command {
    pub fn run(&self, mut cfg: JorupConfig) -> Result<(), Error> {
        let blockchain =
            Blockchain::load(&mut cfg, &self.blockchain).map_err(Error::NoValidBlockchain)?;
        blockchain.prepare().map_err(Error::NoValidBlockchain)?;

        let output = config::Config {
            log: vec![config::Log {
                output: "stderr".to_string(),
                level: "info".to_string(),
                format: "plain".to_string(),
            }],
            p2p: config::P2p {
                public_address: "/ip4/127.0.0.1/tcp/3000".to_string(),
                trusted_peers: blockchain.entry().trusted_peers().to_vec(),
            },
            rest: config::Rest {
                listen: "127.0.0.1:8080".to_string(),
            },
            storage: blockchain.get_node_storage(),
            secret_files: vec![blockchain.get_node_secret()],
        };

        match self.format {
            ConfigFormat::Json => {
                serde_json::to_writer_pretty(std::io::stdout(), &output).map_err(Error::Json)
            }
            ConfigFormat::Yaml => {
                serde_yaml::to_writer(std::io::stdout(), &output).map_err(Error::Yaml)
            }
        }
    }
}

impl std::str::FromStr for ConfigFormat {
    type Err = ConfigFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "json" {
            Ok(Self::Json)
        } else if s == "yaml" {
            Ok(Self::Yaml)
        } else {
            Err(ConfigFormatError)
        }
    }
}
