use chrono::{Date, Utc};
use semver::{Version, VersionReq};
use serde::{de::Deserializer, ser::Serializer, Deserialize, Serialize};
use std::{fmt, str};

/// a testnet entry in the system
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Entry {
    /// the testnet entry channel
    channel: Channel,
    /// description (optional, can be empty)
    description: String,
    /// set the disposition of a given testnet status
    disposition: Disposition,

    /// supported version of the jormungandr's binaries
    jormungandr_versions: VersionReq,
    /// the genesis data (hash and block0 and initial yaml)
    genesis: Genesis,
    /// the list of trusted peers that can be used to connect to the
    /// network.
    known_trusted_peers: Vec<TrustedPeer>,
}

pub struct EntryBuilder {
    channel: Option<Channel>,
    description: Option<String>,
    disposition: Option<Disposition>,
    jormungandr_versions: Option<VersionReq>,
    genesis: Option<Genesis>,
    known_trusted_peers: Vec<TrustedPeer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TrustedPeer {
    address: poldercast::Address,
    id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum Disposition {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Genesis {
    pub block0_hash: String,
    pub block0: String,
    pub content: String,
}

/// a channel:
///
/// * **stable**: `v0.1.2` for example
/// * **nightly**: `v0.1.2-nightly (2019-09-02)`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Channel {
    Stable { version: Version },
    Nightly { version: Version, date: Date<Utc> },
}

const CHANNEL_DATE_FORMAT: &str = " (%F)";

impl EntryBuilder {
    pub fn channel(&mut self, channel: Channel) -> &mut Self {
        self.channel = Some(channel);
        self
    }
    pub fn description<D>(&mut self, description: D) -> &mut Self
    where
        D: Into<String>,
    {
        self.description = Some(description.into());
        self
    }
    pub fn disposition(&mut self, disposition: Disposition) -> &mut Self {
        self.disposition = Some(disposition);
        self
    }
    pub fn jormungandr_versions<I>(&mut self, version: I) -> &mut Self
    where
        I: Into<VersionReq>,
    {
        self.jormungandr_versions = Some(version.into());
        self
    }
    pub fn genesis(&mut self, genesis: Genesis) -> &mut Self {
        self.genesis = Some(genesis);
        self
    }
    pub fn known_trusted_peers<I>(&mut self, trusted_peers: I) -> &mut Self
    where
        I: IntoIterator<Item = TrustedPeer>,
    {
        self.known_trusted_peers = trusted_peers.into_iter().collect();
        self
    }
    pub fn build(&self) -> Entry {
        Entry {
            channel: self
                .channel
                .clone()
                .expect("channel was not set for the entry"),
            description: self
                .description
                .clone()
                .expect("description was not set for the entry"),
            disposition: self
                .disposition
                .clone()
                .expect("disposition was not set for the entry"),
            jormungandr_versions: self
                .jormungandr_versions
                .clone()
                .expect("missing jormungandr's supported versions"),
            genesis: self
                .genesis
                .clone()
                .expect("genesis data were not set for the entry"),
            known_trusted_peers: self.known_trusted_peers.clone(),
        }
    }
}

impl Entry {
    pub fn channel(&self) -> &Channel {
        &self.channel
    }

    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn disposition(&self) -> &Disposition {
        &self.disposition
    }
    pub fn jormungandr_versions(&self) -> &VersionReq {
        &self.jormungandr_versions
    }
    pub fn genesis(&self) -> &Genesis {
        &self.genesis
    }
    pub fn known_trusted_peers(&self) -> &[TrustedPeer] {
        &self.known_trusted_peers
    }
}

impl TrustedPeer {
    pub fn address(&self) -> &poldercast::Address {
        &self.address
    }
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl Genesis {
    /// Block0 hash, in hexadecimal
    pub fn block0_hash(&self) -> &str {
        &self.block0_hash
    }
    /// Block0 in hexadecimal
    pub fn block0(&self) -> &str {
        &self.block0
    }
    /// block0 content genesis file
    pub fn content(&self) -> &str {
        &self.content
    }
}

impl Channel {
    pub fn channel(&self) -> &str {
        match self {
            Self::Nightly { .. } => "nightly",
            Self::Stable { .. } => "stable",
        }
    }

    pub fn version(&self) -> &Version {
        match self {
            Self::Stable { version } => version,
            Self::Nightly { version, .. } => version,
        }
    }

    pub fn is_nightly(&self) -> bool {
        match self {
            Self::Nightly { .. } => true,
            Self::Stable { .. } => false,
        }
    }

    pub fn is_stabler(&self) -> bool {
        !self.is_nightly()
    }

    pub fn nightly_date(&self) -> Option<String> {
        match self {
            Self::Stable { .. } => None,
            Self::Nightly { date, .. } => Some(date.format("%F").to_string()),
        }
    }
}

/* *********************** Default ***************************************** */

impl Default for EntryBuilder {
    fn default() -> EntryBuilder {
        EntryBuilder {
            channel: None,
            description: None,
            disposition: None,
            jormungandr_versions: None,
            genesis: None,
            known_trusted_peers: Vec::new(),
        }
    }
}

/* *********************** Display ***************************************** */

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Channel::Stable { version } => version.fmt(f),
            Channel::Nightly { version, date } => {
                write!(f, "{}-nightly{}", version, date.format(CHANNEL_DATE_FORMAT))
            }
        }
    }
}

/* *********************** FromStr ***************************************** */

error_chain! {
    types {
        ChannelError, ChannelErrorKind, ChannelResult, ChannelResultExt;
    }

    errors {
        MissingVersionComponentOfVersion( s: String ) {
            description("Missing version component of the channel")
            display("Value '{}' is not a valid channel, missing version component", s)
        }

        InvalidVersion(s: String) {
            description("Invalid version component format")
            display("Value '{}' is not a valid channel, invalid version component", s)
        }

        InvalidDate(s: String) {
            description("Invalid date component format")
            display("Value '{}' is not a valid channel, invalid date component", s)
        }

        InvalidChannelFormat(s: String) {
            description("Invalid channel format, too many '-nightly-'"),
            display("Value {} is not a valid channel format, too many '-nightly-'", s),
        }
    }
}

impl str::FromStr for Channel {
    type Err = ChannelError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut sp = s.split("-nightly").filter(|s| !s.is_empty());

        let version = if let Some(s) = sp.next() {
            s.parse()
                .chain_err(|| ChannelErrorKind::InvalidVersion(s.to_owned()))?
        } else {
            bail!(ChannelErrorKind::MissingVersionComponentOfVersion(
                s.to_string()
            ))
        };

        if let Some(s) = sp.next() {
            if sp.count() > 0 {
                bail!(ChannelErrorKind::InvalidChannelFormat(s.to_owned()))
            }

            chrono::naive::NaiveDate::parse_from_str(s, CHANNEL_DATE_FORMAT)
                .chain_err(|| ChannelErrorKind::InvalidDate(s.to_owned()))
                .map(|date| Date::<Utc>::from_utc(date, Utc))
                .map(|date| Channel::Nightly { version, date })
        } else if s.ends_with("-nightly") {
            Ok(Channel::Nightly {
                version,
                date: Utc::today(),
            })
        } else {
            Ok(Channel::Stable { version })
        }
    }
}

/* *********************** Serde ******************************************* */

impl Serialize for Channel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Channel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as _;
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Channel {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let version = Version::new(u64::arbitrary(g), u64::arbitrary(g), u64::arbitrary(g));

            if bool::arbitrary(g) && false {
                Channel::Stable { version }
            } else {
                let date = Utc::today();

                Channel::Nightly { version, date }
            }
        }
    }

    fn unit(s: &str, expected: Channel) {
        let decoded: Channel = s.parse().unwrap();

        assert_eq!(
            expected, decoded,
            "did not decode channel properly from '{}'",
            s
        );
    }

    #[test]
    fn units() {
        unit(
            "0.1.2",
            Channel::Stable {
                version: Version::new(0, 1, 2),
            },
        );
        unit(
            "0.1.2-nightly",
            Channel::Nightly {
                version: Version::new(0, 1, 2),
                date: Utc::today(),
            },
        );
        unit(
            &format!("0.1.2-nightly{}", Utc::today().format(CHANNEL_DATE_FORMAT)),
            Channel::Nightly {
                version: Version::new(0, 1, 2),
                date: Utc::today(),
            },
        );
    }

    #[quickcheck]
    fn channel_serde_json(channel: Channel) -> bool {
        let encoded = serde_json::to_string(&channel).unwrap();
        let decoded: Channel = serde_json::from_str(&encoded).unwrap();

        channel == decoded
    }

    #[quickcheck]
    fn channel_display_from_str(channel: Channel) -> bool {
        let encoded = channel.to_string();
        let decoded: Channel = encoded.parse().unwrap();

        channel == decoded
    }
}
