use semver::VersionReq;
use serde::{de::Deserializer, ser::Serializer, Deserialize, Serialize};
use std::{fmt, str};

/// a testnet entry in the system
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Entry {
    /// the testnet entry channel
    channel: ChannelDesc,
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
    channel: Option<ChannelDesc>,
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
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Stable,
    Beta,
    Nightly,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartialChannelDesc {
    channel: Channel,
    date: Option<Date>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChannelDesc {
    channel: Channel,
    date: Date,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(chrono::Date<chrono::Utc>);

const CHANNEL_DATE_FORMAT: &str = "%F";

impl EntryBuilder {
    pub fn channel(&mut self, channel: ChannelDesc) -> &mut Self {
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
    pub fn channel(&self) -> &ChannelDesc {
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
}

impl Date {
    fn today() -> Self {
        Date(chrono::Utc::today())
    }
}

impl PartialChannelDesc {
    pub fn channel(&self) -> &Channel {
        &self.channel
    }

    pub fn date(&self) -> Option<&Date> {
        self.date.as_ref()
    }

    pub fn matches(&self, channel_desc: &ChannelDesc) -> bool {
        if self.channel() == channel_desc.channel() {
            if let Some(date) = self.date() {
                date == channel_desc.date()
            } else {
                true
            }
        } else {
            false
        }
    }

    pub fn into_channel_desc(self) -> ChannelDesc {
        let channel = self.channel;
        let date = self.date.unwrap_or_else(|| Date::today());

        ChannelDesc { channel, date }
    }
}

impl ChannelDesc {
    pub fn channel(&self) -> &Channel {
        &self.channel
    }

    pub fn date(&self) -> &Date {
        &self.date
    }

    pub fn is_nightly(&self) -> bool {
        self.channel() == &Channel::Nightly
    }

    pub fn is_beta(&self) -> bool {
        self.channel() == &Channel::Beta
    }

    pub fn is_stable(&self) -> bool {
        self.channel() == &Channel::Stable
    }
}

/* *********************** Default ***************************************** */

impl Default for PartialChannelDesc {
    fn default() -> Self {
        PartialChannelDesc {
            channel: Channel::Stable,
            date: None,
        }
    }
}

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

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.format(CHANNEL_DATE_FORMAT))
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Channel::Stable => "stable".fmt(f),
            Channel::Beta => "beta".fmt(f),
            Channel::Nightly => "nightly".fmt(f),
        }
    }
}

impl fmt::Display for ChannelDesc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}", self.channel(), self.date)
    }
}

impl fmt::Display for PartialChannelDesc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.channel())?;

        if let Some(date) = self.date() {
            write!(f, "-{}", date)?;
        }

        Ok(())
    }
}

/* *********************** FromStr ***************************************** */

error_chain! {
    types {
        ChannelError, ChannelErrorKind, ChannelResult, ChannelResultExt;
    }

    errors {
        InvalidDate(s: String) {
            description("Invalid date component format")
            display("Value '{}' is not a valid date, invalid date component", s)
        }
    }
}

impl str::FromStr for PartialChannelDesc {
    type Err = ChannelError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(i) = s.find('-') {
            let channel = s[..i].parse()?;
            let date = Some(s[i + 1..].parse()?);
            Ok(PartialChannelDesc { channel, date })
        } else {
            let channel = s.parse()?;
            let date = None;
            Ok(PartialChannelDesc { channel, date })
        }
    }
}

impl str::FromStr for Date {
    type Err = ChannelError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        chrono::naive::NaiveDate::parse_from_str(&s, CHANNEL_DATE_FORMAT)
            .chain_err(|| ChannelErrorKind::InvalidDate(s.to_owned()))
            .map(|date| chrono::Date::<chrono::Utc>::from_utc(date, chrono::Utc))
            .map(Date)
    }
}

impl str::FromStr for Channel {
    type Err = ChannelError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "stable" {
            Ok(Channel::Stable)
        } else if s == "beta" {
            Ok(Channel::Beta)
        } else if s == "nightly" {
            Ok(Channel::Nightly)
        } else {
            bail!(format!("Invalid channel: {}", s))
        }
    }
}

/* *********************** Serde ******************************************* */

impl Serialize for Date {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0
            .format(CHANNEL_DATE_FORMAT)
            .to_string()
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as _;
        let s = String::deserialize(deserializer)?;
        chrono::naive::NaiveDate::parse_from_str(&s, CHANNEL_DATE_FORMAT)
            .chain_err(|| format!("Invalid date: {}", s))
            .map_err(D::Error::custom)
            .map(|date| chrono::Date::<chrono::Utc>::from_utc(date, chrono::Utc))
            .map(Date)
    }
}

impl Serialize for PartialChannelDesc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PartialChannelDesc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error as _;
        let s = String::deserialize(deserializer)?;
        s.parse::<PartialChannelDesc>().map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Channel {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            match u8::arbitrary(g) % 3 {
                0 => Channel::Stable,
                1 => Channel::Beta,
                _ => Channel::Nightly,
            }
        }
    }

    impl Arbitrary for Date {
        fn arbitrary<G: Gen>(_g: &mut G) -> Self {
            Date(chrono::Utc::now().date())
        }
    }

    impl Arbitrary for ChannelDesc {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            ChannelDesc {
                channel: Channel::arbitrary(g),
                date: Date::arbitrary(g),
            }
        }
    }

    impl Arbitrary for PartialChannelDesc {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            PartialChannelDesc {
                channel: Channel::arbitrary(g),
                date: if bool::arbitrary(g) {
                    Some(Date::arbitrary(g))
                } else {
                    None
                },
            }
        }
    }

    fn unit_from_str<T>(s: &str, expected: T)
    where
        T: str::FromStr + Eq + fmt::Debug,
        <T as str::FromStr>::Err: fmt::Debug,
    {
        let decoded: T = s.parse().unwrap();

        assert_eq!(
            expected, decoded,
            "did not decode channel properly from '{}'",
            s
        );
    }

    #[test]
    fn channel_units() {
        unit_from_str("stable", Channel::Stable);
        unit_from_str("beta", Channel::Beta);
        unit_from_str("nightly", Channel::Nightly);
    }

    #[test]
    fn date_units() {
        use chrono::{NaiveDate, Utc};
        unit_from_str(
            "1920-10-14",
            Date(chrono::Date::from_utc(
                NaiveDate::from_ymd(1920, 10, 14),
                Utc,
            )),
        );
        unit_from_str(
            "2019-02-24",
            Date(chrono::Date::from_utc(
                NaiveDate::from_ymd(2019, 02, 24),
                Utc,
            )),
        );
    }

    #[test]
    fn partial_channel_desc_units() {
        use chrono::{NaiveDate, Utc};
        unit_from_str(
            "stable",
            PartialChannelDesc {
                channel: Channel::Stable,
                date: None,
            },
        );
        unit_from_str(
            "beta",
            PartialChannelDesc {
                channel: Channel::Beta,
                date: None,
            },
        );
        unit_from_str(
            "nightly",
            PartialChannelDesc {
                channel: Channel::Nightly,
                date: None,
            },
        );

        unit_from_str(
            "stable-1979-12-10",
            PartialChannelDesc {
                channel: Channel::Stable,
                date: Some(Date(chrono::Date::from_utc(
                    NaiveDate::from_ymd(1979, 12, 10),
                    Utc,
                ))),
            },
        );
        unit_from_str(
            "beta-2000-01-01",
            PartialChannelDesc {
                channel: Channel::Beta,
                date: Some(Date(chrono::Date::from_utc(
                    NaiveDate::from_ymd(2000, 01, 01),
                    Utc,
                ))),
            },
        );
        unit_from_str(
            "nightly-2021-08-31",
            PartialChannelDesc {
                channel: Channel::Nightly,
                date: Some(Date(chrono::Date::from_utc(
                    NaiveDate::from_ymd(2021, 08, 31),
                    Utc,
                ))),
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

    #[quickcheck]
    fn date_serde_json(date: Date) -> bool {
        let encoded = serde_json::to_string(&date).unwrap();
        let decoded: Date = serde_json::from_str(&encoded).unwrap();

        date == decoded
    }

    #[quickcheck]
    fn date_display_from_str(date: Date) -> bool {
        let encoded = date.to_string();
        let decoded: Date = encoded.parse().unwrap();

        date == decoded
    }

    #[quickcheck]
    fn channel_desc_serde_json(channel_desc: ChannelDesc) -> bool {
        let encoded = serde_json::to_string(&channel_desc).unwrap();
        let decoded: ChannelDesc = serde_json::from_str(&encoded).unwrap();

        channel_desc == decoded
    }

    #[quickcheck]
    fn partial_channel_desc_display_from_str(partial_channel_desc: PartialChannelDesc) -> bool {
        let encoded = partial_channel_desc.to_string();
        let decoded: PartialChannelDesc = encoded.parse().unwrap();

        partial_channel_desc == decoded
    }
}
