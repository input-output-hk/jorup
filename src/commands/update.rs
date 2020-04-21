use crate::{
    common::JorupConfig,
    utils::{blockchain::Blockchain, download_file, release::Release},
};
use structopt::StructOpt;
use thiserror::Error;

/// Sync and update the local blockchain
#[derive(Debug, StructOpt)]
pub struct Command {
    /// The blockchain to run jormungandr for
    blockchain: String,

    /// Make the associated jormungandr release the default
    #[structopt(long)]
    make_default: bool,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error while syncing releases and blockchains, no internet? try `--offline`...")]
    SyncFailed(#[source] crate::common::Error),
    #[error("Cannot run the node without valid blockchain")]
    NoValidBlockchain(#[source] crate::utils::blockchain::Error),
    #[error("Cannot run without compatible release")]
    NoCompatibleRelease(#[source] crate::utils::release::Error),
    #[error("Cannot download and install an update")]
    CannotUpdate(#[source] crate::utils::download::Error),
}

impl Command {
    pub fn run(self, mut cfg: JorupConfig) -> Result<(), Error> {
        cfg.sync_jorfile().map_err(Error::SyncFailed)?;

        // prepare entry directory
        let blockchain =
            Blockchain::load(&mut cfg, &self.blockchain).map_err(Error::NoValidBlockchain)?;
        blockchain.prepare().map_err(Error::NoValidBlockchain)?;
        let release = Release::new(&mut cfg, blockchain.jormungandr_version_req())
            .map_err(Error::NoCompatibleRelease)?;
        let asset = release.asset_remote().map_err(Error::NoCompatibleRelease)?;

        if release.asset_need_fetched() && !cfg.offline() {
            download_file(
                &release.get_asset().display().to_string(),
                &asset.as_ref(),
                release.get_asset(),
            )
            .map_err(Error::CannotUpdate)?;
            println!("**** asset downloaded");
        }

        release.asset_open().map_err(Error::NoCompatibleRelease)?;

        if self.make_default {
            release
                .make_default(&cfg)
                .map_err(Error::NoCompatibleRelease)?;
        }

        println!("**** jormungandr updated to version {}", release.version());
        Ok(())
    }
}
