use chrono::{offset::Utc, DateTime};
use semver::{Version as SemverVersion, VersionReq as SemverVersionReq};
use serde::{de, Deserialize, Deserializer};
use std::{
    cmp::{Ordering, PartialOrd},
    fmt,
    str::FromStr,
};

pub use semver::{ReqParseError, SemVerError};

const DATEFMT: &str = "%Y%m%d";

#[derive(Debug, Clone, PartialEq, Eq, Ord)]
pub enum Version {
    Nightly(Option<(SemverVersion, DateTime<Utc>)>),
    Stable(SemverVersion),
}

#[derive(Debug, Clone)]
pub enum VersionReq {
    Latest,
    Nightly,
    Stable(SemverVersionReq),
    ExactStable(SemverVersion),
}

impl Version {
    pub fn parse(version: &str) -> Result<Self, SemVerError> {
        if version == "nightly" {
            return Ok(Version::Nightly(None));
        }
        SemverVersion::parse(version).map(Version::Stable)
    }

    pub fn from_git_tag(version: &str) -> Result<Self, SemVerError> {
        let version = version.trim_start_matches('v');
        Self::parse(version)
    }

    pub fn to_git_tag(&self) -> String {
        match self {
            Version::Nightly(_) => "nightly".to_string(),
            Version::Stable(version) => format!("v{}", version),
        }
    }

    pub fn to_version_number(&self) -> String {
        match self {
            Version::Nightly(None) => panic!("unconfigured nightly"),
            Version::Nightly(Some((version, datetime))) => {
                format!("{}-nightly.{}", version, datetime.format(DATEFMT))
            }
            Version::Stable(version) => format!("v{}", version),
        }
    }

    pub fn configure_nightly(self, last_stable_version: Self, datetime: DateTime<Utc>) -> Self {
        let mut version = match last_stable_version {
            Version::Stable(version) => version,
            Version::Nightly(_) => panic!("only Stable can be provided to this method"),
        };
        version.increment_patch();
        match self {
            Version::Nightly(_) => Version::Nightly(Some((version, datetime))),
            v => v,
        }
    }
}

impl VersionReq {
    pub fn parse(version_req: &str) -> Result<Self, ReqParseError> {
        if version_req == "nightly" {
            return Ok(VersionReq::Nightly);
        }
        SemverVersionReq::parse(version_req).map(VersionReq::Stable)
    }

    pub fn exact(version: Version) -> Self {
        match version {
            Version::Nightly(_) => VersionReq::Nightly,
            Version::Stable(version) => VersionReq::ExactStable(version),
        }
    }

    pub fn matches(&self, version: &Version) -> bool {
        match self {
            VersionReq::Latest => false,
            VersionReq::Nightly => match version {
                Version::Nightly(_) => true,
                _ => false,
            },
            VersionReq::Stable(version_req) => match version {
                Version::Nightly(_) => false,
                Version::Stable(version) => version_req.matches(version),
            },
            VersionReq::ExactStable(version_req) => match version {
                Version::Nightly(_) => false,
                Version::Stable(other) => version_req.eq(other),
            },
        }
    }

    pub fn into_version(self) -> Option<Version> {
        if let VersionReq::ExactStable(version) = self {
            return Some(Version::Stable(version));
        }
        None
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VersionVisitor;

        // Deserialize Version from a string.
        impl<'de> de::Visitor<'de> for VersionVisitor {
            type Value = Version;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a version as a string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Version::parse(v).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(VersionVisitor)
    }
}

impl<'de> Deserialize<'de> for VersionReq {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VersionReqVisitor;

        // Deserialize Version from a string.
        impl<'de> de::Visitor<'de> for VersionReqVisitor {
            type Value = VersionReq;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a version requirement as a string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                VersionReq::parse(v).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(VersionReqVisitor)
    }
}

impl FromStr for Version {
    type Err = SemVerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Version::parse(s)
    }
}

impl FromStr for VersionReq {
    type Err = ReqParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        VersionReq::parse(s)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Version::Nightly(None) => f.write_str("nightly"),
            Version::Nightly(Some((version, datetime))) => {
                write!(f, "{}-nightly.{}", version, datetime.format(DATEFMT))
            }
            Version::Stable(version) => f.write_str(&version.to_string()),
        }
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionReq::Latest => f.write_str("latest"),
            VersionReq::Nightly => f.write_str("nightly"),
            VersionReq::Stable(version_req) => f.write_str(&version_req.to_string()),
            VersionReq::ExactStable(version) => f.write_str(&version.to_string()),
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let res = match self {
            Version::Nightly(datetime) => match other {
                Version::Nightly(other_datetime) => return datetime.partial_cmp(other_datetime),
                Version::Stable(_) => Ordering::Less,
            },
            Version::Stable(version) => match other {
                Version::Nightly(_) => Ordering::Greater,
                Version::Stable(other) => version.cmp(other),
            },
        };
        Some(res)
    }
}
