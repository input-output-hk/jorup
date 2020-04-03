#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;
#[macro_use(error_chain, bail)]
extern crate error_chain;

mod download;
mod jormungandr;
mod testnet;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use download::download;
pub use jormungandr::{Release, ReleaseBuilder, Url, UrlBuilder, AVAILABLE_PLATFORMS};
pub use platforms::Platform;
pub use semver::{Version, VersionReq};
pub use testnet::{
    Channel, ChannelDesc, ChannelError, ChannelErrorKind, Date, Disposition, Entry, EntryBuilder,
    Genesis, PartialChannelDesc, TrustedPeer,
};

error_chain! {
    errors {
        ReleaseConflict (previous_release: Version) {
            description("Release already exists"),
            display("Version '{}' already exists", previous_release),
        }

        EntryConflict (previous_channel: ChannelDesc) {
            description("Entry already exists"),
            display("Channel '{}' already exists", previous_channel),
        }

        NoCompatibleVersions (version_req: VersionReq, versions: Vec<String>) {
            description("No releases matches the version requirements"),
            display("No release versions to supports requirements ({}). Available ones: {:?}", version_req, versions),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(remote = "JorData")]
struct JorDataDef {
    #[serde(getter = "JorData::releases")]
    releases: Vec<Release>,
    #[serde(getter = "JorData::entries")]
    entries: Vec<Entry>,
}

#[derive(Debug)]
pub struct JorData {
    releases: BTreeMap<Version, Release>,
    entries: BTreeMap<ChannelDesc, Entry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Jor(#[serde(with = "JorDataDef")] JorData);

impl Jor {
    pub fn releases(&self) -> &BTreeMap<Version, Release> {
        &self.0.releases
    }

    pub fn entries(&self) -> &BTreeMap<ChannelDesc, Entry> {
        &self.0.entries
    }

    pub fn search_release(&self, version_req: VersionReq) -> Option<&Release> {
        self.releases()
            .values()
            .filter(|release| version_req.matches(release.version()))
            .last()
    }

    pub fn search_entry(&self, nightly: bool, version_req: PartialChannelDesc) -> Option<&Entry> {
        self.entries()
            .values()
            .filter(|entry| entry.channel().is_nightly() == nightly)
            .filter(|entry| version_req.matches(entry.channel()))
            .last()
    }

    pub fn remove_release(&mut self, release: &Version) -> Option<Release> {
        self.0.releases.remove(release)
    }

    pub fn add_release(&mut self, release: Release) -> Result<()> {
        if let Some(prev) = self.0.releases.insert(release.version().clone(), release) {
            bail!(ErrorKind::ReleaseConflict(prev.version().clone()))
        } else {
            Ok(())
        }
    }

    pub fn add_entry(&mut self, entry: Entry) -> Result<()> {
        let version_req = entry.jormungandr_versions();

        let at_least_one = self
            .releases()
            .values()
            .any(|release| version_req.matches(release.version()));

        if !at_least_one {
            bail!(ErrorKind::NoCompatibleVersions(
                version_req.clone(),
                self.releases()
                    .values()
                    .map(|r| r.version().to_string())
                    .collect(),
            ))
        }

        if let Some(prev) = self.0.entries.insert(entry.channel().clone(), entry) {
            bail!(ErrorKind::EntryConflict(prev.channel().clone()))
        } else {
            Ok(())
        }
    }
}

impl JorData {
    fn releases(&self) -> Vec<Release> {
        self.releases.values().cloned().collect()
    }

    fn entries(&self) -> Vec<Entry> {
        self.entries.values().cloned().collect()
    }
}

impl Default for Jor {
    fn default() -> Self {
        Jor(JorData {
            releases: BTreeMap::new(),
            entries: BTreeMap::new(),
        })
    }
}

impl From<JorDataDef> for JorData {
    fn from(data_def: JorDataDef) -> JorData {
        JorData {
            releases: data_def
                .releases
                .into_iter()
                .map(|release| (release.version().clone(), release))
                .collect(),
            entries: data_def
                .entries
                .into_iter()
                .map(|entry| (entry.channel().clone(), entry))
                .collect(),
        }
    }
}
