use crate::{
    common::JorupConfig,
    utils::{
        blockchain::Blockchain,
        jcli::Jcli,
        release::Release,
        version::{Version, VersionReq},
    },
};
use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

/// Wallet operations
#[derive(Debug, StructOpt)]
pub struct Command {
    /// The blockchain to run jormungandr for
    blockchain: String,

    /// The version of Jormungandr to run. If not specified, the latest
    /// compatible version will be used.
    #[structopt(short, long)]
    version: Option<Version>,

    /// The directory containing jormungandr and jcli, can be useful for
    /// development purposes. When provided, the `--version` flag is ignored.
    #[structopt(long)]
    bin: Option<PathBuf>,

    /// Force re-creating a wallet if it does exists already
    #[structopt(long)]
    force_create_wallet: bool,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot run the node without valid blockchain")]
    NoValidBlockchain(#[source] crate::utils::blockchain::Error),
    #[error("Cannot run without compatible release")]
    NoCompatibleRelease(#[source] crate::utils::release::Error),
    #[error("No binaries for this blockchain")]
    NoCompatibleBinaries,
    #[error("Cannot create new wallet")]
    CannotCreateWallet(#[source] crate::utils::jcli::Error),
    #[error("Cannot get the wallet's address")]
    CannotGetAddress(#[source] crate::utils::jcli::Error),
}

impl Command {
    pub fn run(self, mut cfg: JorupConfig) -> Result<(), Error> {
        // prepare entry directory
        let blockchain =
            Blockchain::load(&mut cfg, &self.blockchain).map_err(Error::NoValidBlockchain)?;
        blockchain.prepare().map_err(Error::NoValidBlockchain)?;

        let bin = if let Some(dir) = self.bin {
            eprintln!("WARN: using custom binaries from {}", dir.display());
            dir.join("jcli")
        } else {
            let release = if let Some(version) = self.version {
                Release::load(&mut cfg, &VersionReq::exact(version))
            } else {
                Release::load(&mut cfg, blockchain.jormungandr_version_req())
            }
            .map_err(|err| {
                eprintln!("HINT: run `jorup node install`");
                Error::NoCompatibleRelease(err)
            })?;

            if release.asset_need_fetched() {
                // asset release is not available
                return Err(Error::NoCompatibleBinaries);
            }

            release.dir().join("jcli")
        };

        let mut runner = Jcli::new(&blockchain, bin);

        runner
            .get_wallet_secret_key(self.force_create_wallet)
            .map_err(Error::CannotCreateWallet)?;
        let address = runner
            .get_wallet_address()
            .map_err(Error::CannotGetAddress)?;

        println!("Wallet: {}", address);

        Ok(())
    }
}
