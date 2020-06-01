use crate::{
    common::JorupConfig,
    utils::{blockchain::Blockchain, jcli::Jcli, release::Release, version::VersionReq},
};
use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

/// Generate a wallet if it does not exist. Output the public key, address, and
/// secret key path.
#[derive(Debug, StructOpt)]
pub struct Command {
    /// The blockchain to run jormungandr for
    blockchain: String,

    /// Address prefix (ignored by node, exists for readability, default: jorup_)
    #[structopt(default_value = "jorup_")]
    prefix: String,

    /// The version of Jormungandr to run. If not specified, the latest
    /// compatible version will be used.
    #[structopt(short = "v", long = "version")]
    version_req: Option<VersionReq>,

    /// The directory containing jormungandr and jcli, can be useful for
    /// development purposes. When provided, the `--version` flag is ignored.
    #[structopt(long)]
    bin: Option<PathBuf>,

    /// Path to an existing secret key (will be copied to the wallet directory).
    /// `force_create_wallet` will be ignored. WARNING: this will overwrite the
    /// existing key.
    #[structopt(long)]
    import: Option<PathBuf>,

    /// Force re-creating a wallet if it does exist already
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
    #[error("Cannot get the wallet's public key")]
    CannotGetPublicKey(#[source] crate::utils::jcli::Error),
    #[error("An error occurred while importing a wallet")]
    ImportError(#[source] std::io::Error),
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
            let release = if let Some(version_req) = self.version_req {
                Release::load(&mut cfg, &version_req)
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
        let sk_path = runner.get_wallet_secret_key_path();

        if let Some(import_sk_path) = self.import {
            let confirm_overwrite = dialoguer::Confirmation::new()
                .with_text("This will overwrite the existing private key. Continue?")
                .interact()
                .unwrap();
            if confirm_overwrite {
                std::fs::copy(import_sk_path, &sk_path).map_err(Error::ImportError)?;
            }
        } else {
            if !sk_path.is_file() || self.force_create_wallet {
                runner
                    .generate_wallet_secret_key()
                    .map_err(Error::CannotCreateWallet)?;
            }
        }

        let public_key = runner.get_public_key().map_err(Error::CannotGetPublicKey)?;
        let address = runner
            .get_wallet_address(&self.prefix)
            .map_err(Error::CannotGetAddress)?;

        println!("Secret key: {}", sk_path.display());
        println!("Public key: {}", public_key);
        println!("Wallet: {}", address);

        Ok(())
    }
}
