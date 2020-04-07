#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;
#[macro_use(error_chain, bail)]
extern crate error_chain;

mod download;
mod testnet;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use download::download;
pub use testnet::{
    Channel, ChannelDesc, ChannelError, ChannelErrorKind, Date, Disposition, Entry, EntryBuilder,
    Genesis, PartialChannelDesc, TrustedPeer,
};

error_chain! {
    errors {
        EntryConflict (previous_channel: ChannelDesc) {
            description("Entry already exists"),
            display("Channel '{}' already exists", previous_channel),
        }
    }
}

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

    pub fn add_entry(&mut self, entry: Entry) -> Result<()> {
        if let Some(prev) = self.0.entries.insert(entry.channel().clone(), entry) {
            bail!(ErrorKind::EntryConflict(prev.channel().clone()))
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
