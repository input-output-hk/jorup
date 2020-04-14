mod testnet;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

pub use testnet::{
    Channel, ChannelDesc, Date, Disposition, Entry, EntryBuilder, Genesis, PartialChannelDesc,
    TrustedPeer,
};

#[derive(Debug, Error)]
#[error("Channel '{0}' already exists")]
struct Error(ChannelDesc);

#[derive(Debug, Serialize, Deserialize)]
#[serde(remote = "JorData")]
struct JorDataDef {
    #[serde(getter = "JorData::entries")]
    entries: Vec<Entry>,
}

#[derive(Debug)]
pub struct JorData {
    entries: BTreeMap<ChannelDesc, Entry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Jor(#[serde(with = "JorDataDef")] JorData);

impl Jor {
    pub fn entries(&self) -> &BTreeMap<ChannelDesc, Entry> {
        &self.0.entries
    }

    pub fn search_entry(&self, nightly: bool, version_req: PartialChannelDesc) -> Option<&Entry> {
        self.entries()
            .values()
            .filter(|entry| entry.channel().is_nightly() == nightly)
            .filter(|entry| version_req.matches(entry.channel()))
            .last()
    }

    pub fn add_entry(&mut self, entry: Entry) -> Result<(), Error> {
        if let Some(prev) = self.0.entries.insert(entry.channel().clone(), entry) {
            Err(Error(prev.channel().clone()))
        } else {
            Ok(())
        }
    }
}

impl JorData {
    fn entries(&self) -> Vec<Entry> {
        self.entries.values().cloned().collect()
    }
}

impl Default for Jor {
    fn default() -> Self {
        Jor(JorData {
            entries: BTreeMap::new(),
        })
    }
}

impl From<JorDataDef> for JorData {
    fn from(data_def: JorDataDef) -> JorData {
        JorData {
            entries: data_def
                .entries
                .into_iter()
                .map(|entry| (entry.channel().clone(), entry))
                .collect(),
        }
    }
}
