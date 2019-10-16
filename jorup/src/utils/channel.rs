use crate::common::JorupConfig;
use error_chain::ChainedError as _;
use jorup_lib::VersionReq;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

error_chain! {}

#[derive(Debug, Clone)]
pub enum ChannelVersion {
    Stable,
    Nightly,
    Specific { channel: jorup_lib::Channel },
}

pub struct Channel {
    entry: jorup_lib::Entry,
    version: ChannelVersion,

    path: PathBuf,
}

impl ChannelVersion {
    pub fn is_nightly(&self) -> bool {
        match self {
            Self::Nightly => true,
            Self::Stable => false,
            Self::Specific { channel } => channel.is_nightly(),
        }
    }
}

const CHANNEL_NAME: &str = "CHANNEL_NAME";

impl Channel {
    pub fn arg<'a, 'b>() -> clap::Arg<'a, 'b>
    where
        'a: 'b,
    {
        clap::Arg::with_name(CHANNEL_NAME)
            .value_name("CHANNEL")
            .help("The channel to run jormungandr for, jorup uses the default channel otherwise")
            .validator(|s: String| {
                s.parse::<ChannelVersion>()
                    .map(|_channel| ())
                    .map_err(|err| err.display_chain().to_string())
            })
    }

    pub fn load<'a, 'b>(cfg: &'b mut JorupConfig, args: &clap::ArgMatches<'a>) -> Result<Self> {
        let mut channel_entered = cfg.current_channel().clone();

        let entry = if let Some(channel) = args.value_of(CHANNEL_NAME) {
            let jor = cfg
                .load_jor()
                .chain_err(|| "No jorfile... cannot operate")?;

            // should be save to unwrap as we have set a validator in the Argument
            // for the CLI to check it is valid
            channel_entered = channel.parse().unwrap();
            let entry = match channel.parse().unwrap() {
                ChannelVersion::Nightly => jor.search_entry(true, VersionReq::any()),
                ChannelVersion::Stable => jor.search_entry(false, VersionReq::any()),
                ChannelVersion::Specific { channel } => jor.entries().get(&channel),
            };

            entry.map(|entry| entry.clone())
        } else {
            cfg.current_entry()
                .chain_err(|| "No jorfile... cannot operate")?
                .map(|entry| entry.clone())
        };

        if let Some(entry) = entry {
            Self::new(cfg, entry.clone(), channel_entered)
        } else {
            bail!("No entry available for the given version")
        }
    }

    fn new(
        cfg: &JorupConfig,
        entry: jorup_lib::Entry,
        channel_version: ChannelVersion,
    ) -> Result<Self> {
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
        Ok(Channel {
            entry,
            version: channel_version,
            path,
        })
    }

    pub fn channel_version(&self) -> &ChannelVersion {
        &self.version
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

    pub fn jormungandr_version_req(&self) -> &VersionReq {
        self.entry().jormungandr_versions()
    }

    pub fn entry(&self) -> &jorup_lib::Entry {
        &self.entry
    }

    pub fn get_log_file(&self) -> PathBuf {
        self.dir().join("NODE.logs")
    }

    pub fn get_runner_file(&self) -> PathBuf {
        self.dir().join("running_config.toml")
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

impl std::str::FromStr for ChannelVersion {
    type Err = <jorup_lib::Channel as std::str::FromStr>::Err;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "stable" => Ok(ChannelVersion::Stable),
            "nightly" => Ok(ChannelVersion::Nightly),
            s => Ok(ChannelVersion::Specific {
                channel: s.parse()?,
            }),
        }
    }
}
impl std::fmt::Display for ChannelVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ChannelVersion::Stable => "stable".fmt(f),
            ChannelVersion::Nightly => "nightly".fmt(f),
            ChannelVersion::Specific { channel } => channel.fmt(f),
        }
    }
}

impl Serialize for ChannelVersion {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ChannelVersion {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use serde::de::Error as _;
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(D::Error::custom)
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
