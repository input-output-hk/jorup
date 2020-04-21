use crate::{
    common::JorupConfig,
    utils::{download_file, release::Release},
};
use semver::{Version, VersionReq};
use structopt::StructOpt;
use thiserror::Error;

/// Manage Jormungandr versions
#[derive(Debug, StructOpt)]
pub enum Command {
    /// Install the specified version of Jorumngandr. If no version or
    /// blockchain was specified it will download the latest stable version.
    Install {
        /// Install a particular version of Jormungandr. Cannot be used
        /// alongside --blockchain
        #[structopt(short, long)]
        version: Option<Version>,

        /// Install the latest version compatible with the specified blockchain
        #[structopt(short, long)]
        blockchain: Option<String>,

        /// Make the installed version default
        #[structopt(long)]
        make_default: bool,
    },
    List,
    Remove,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot run this command offline")]
    Offline,
    #[error("Blockchain not found")]
    BlockchainNotFound,
    #[error("Cannot specify blockchain and version at the same time")]
    MustNotSpecifyBlockchainAndVersion,
    #[error("Cannot find any matching releases")]
    NoCompatibleRelease(#[source] crate::utils::release::Error),
    #[error("Cannot download and install an update")]
    CannotUpdate(#[source] crate::utils::download::Error),
}

impl Command {
    pub fn run(self, cfg: JorupConfig) -> Result<(), Error> {
        match self {
            Command::Install {
                version,
                blockchain,
                make_default,
            } => install(cfg, version, blockchain, make_default),
            Command::List => list(),
            Command::Remove => remove(),
        }
    }
}

fn install(
    mut cfg: JorupConfig,
    version: Option<Version>,
    blockchain: Option<String>,
    make_default: bool,
) -> Result<(), Error> {
    if cfg.offline() {
        return Err(Error::Offline);
    }

    if version.is_some() && blockchain.is_some() {
        return Err(Error::MustNotSpecifyBlockchainAndVersion);
    }

    let version_req = match version {
        None => match blockchain {
            None => VersionReq::any(),
            Some(blockchain_name) => match cfg.get_blockchain(&blockchain_name) {
                None => return Err(Error::BlockchainNotFound),
                Some(blockchain) => blockchain.jormungandr_versions().clone(),
            },
        },
        Some(version) => VersionReq::exact(&version),
    };

    let release = Release::new(&mut cfg, &version_req).map_err(Error::NoCompatibleRelease)?;
    let asset = release.asset_remote().map_err(Error::NoCompatibleRelease)?;

    if release.asset_need_fetched() {
        download_file(
            &release.get_asset().display().to_string(),
            &asset.as_ref(),
            release.get_asset(),
        )
        .map_err(Error::CannotUpdate)?;
        println!("**** asset downloaded");
    }

    release.asset_open().map_err(Error::NoCompatibleRelease)?;

    if make_default {
        release
            .make_default(&cfg)
            .map_err(Error::NoCompatibleRelease)?;
    }

    Ok(())
}

fn list() -> Result<(), Error> {
    Ok(())
}

fn remove() -> Result<(), Error> {
    Ok(())
}
