use chrono::{offset::Utc, Date};
use semver::{
    ReqParseError, SemVerError, Version as SemverVersion, VersionReq as SemverVersionReq,
};
use serde::{de, Deserialize, Deserializer};
use std::{
    cmp::{Ordering, PartialOrd},
    fmt,
    str::FromStr,
};
use thiserror::Error;

const DATEFMT: &str = "%Y%m%d";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Version {
    Nightly(Option<Date<Utc>>),
    Stable(SemverVersion),
}

#[derive(Debug, Clone)]
pub enum VersionReq {
    Latest,
    Nightly,
    Stable(SemverVersionReq),
    ExactStable(SemverVersion),
}

#[derive(Debug, Error)]
pub enum VersionError {
    #[error(transparent)]
    Semver(#[from] SemVerError),
    #[error("invalid nightly version")]
    Nightly,
}

impl Version {
    pub fn parse(version: &str) -> Result<Self, VersionError> {
        if version == "nightly" {
            return Ok(Version::Nightly(None));
        }

        if version.starts_with("nightly") {
            let mut parts = version.splitn(2, '.');
            let date: u32 = parts
                .nth(1)
                .ok_or(VersionError::Nightly)?
                .parse()
                .map_err(|_| VersionError::Nightly)?;
            let naive_date = chrono::NaiveDate::from_ymd(
                (date / 10000) as i32,
                (date % 10000 / 100) as u32,
                (date % 100) as u32,
            );
            let date = Date::from_utc(naive_date, Utc);
            return Ok(Version::Nightly(Some(date)));
        }

        SemverVersion::parse(version)
            .map_err(Into::into)
            .map(Version::Stable)
    }

    pub fn from_git_tag(version: &str) -> Result<Self, VersionError> {
        let version = version.trim_start_matches('v');
        Self::parse(version)
    }

    pub fn to_git_tag(&self) -> String {
        match self {
            Version::Nightly(_) => "nightly".to_string(),
            Version::Stable(version) => format!("v{}", version),
        }
    }

    pub fn configure_nightly(self, date: Date<Utc>) -> Self {
        match self {
            Version::Nightly(_) => Version::Nightly(Some(date)),
            v => v,
        }
    }

    pub fn get_nightly_date(&self) -> Option<&Date<Utc>> {
        if let Version::Nightly(maybe_date) = &self {
            maybe_date.as_ref()
        } else {
            None
        }
    }
}

#[derive(Debug, Error)]
pub enum VersionReqError {
    #[error(transparent)]
    ReqError(#[from] ReqParseError),
    #[error(transparent)]
    VersionError(#[from] SemVerError),
}

impl VersionReq {
    pub fn parse(version_req: &str) -> Result<Self, VersionReqError> {
        if version_req == "nightly" {
            return Ok(VersionReq::Nightly);
        }
        if version_req
            .chars()
            .next()
            .map(|c| c.is_numeric())
            .unwrap_or(false)
        {
            return Ok(VersionReq::ExactStable(SemverVersion::parse(version_req)?));
        }
        SemverVersionReq::parse(version_req)
            .map(VersionReq::Stable)
            .map_err(Into::into)
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
            VersionReq::Nightly => matches!(version, Version::Nightly(_)),
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
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Version::parse(s)
    }
}

impl FromStr for VersionReq {
    type Err = VersionReqError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        VersionReq::parse(s)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Version::Nightly(None) => f.write_str("nightly"),
            Version::Nightly(Some(datetime)) => write!(f, "nightly.{}", datetime.format(DATEFMT)),
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

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
