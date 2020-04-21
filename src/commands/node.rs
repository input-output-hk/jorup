use crate::{
    common::JorupConfig,
    utils::{
        blockchain::Blockchain,
        download_file, github,
        release::{Error as ReleaseError, Release},
    },
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
    #[error("Cannot load the requested blockchain")]
    NoValidBlockchain(#[from] crate::utils::blockchain::Error),
    #[error("Cannot find a release on GitHub")]
    GitHub(#[from] crate::utils::github::Error),
    #[error("Cannot specify blockchain and version at the same time")]
    MustNotSpecifyBlockchainAndVersion,
    #[error("Failed to load a release")]
    ReleaseLoadError(#[source] ReleaseError),
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

    let load_latest = version.is_none() && blockchain.is_none();

    let version_req = match version {
        None => match blockchain {
            None => VersionReq::any(),
            Some(blockchain_name) => Blockchain::load(&mut cfg, &blockchain_name)?
                .jormungandr_version_req()
                .clone(),
        },
        Some(version) => VersionReq::exact(&version),
    };

    let release = if load_latest {
        let gh_release = github::find_matching_release(&version_req)?;
        Release::new(&mut cfg, gh_release.version().clone()).map_err(Error::ReleaseLoadError)?
    } else {
        match Release::load(&mut cfg, &version_req) {
            Ok(release) => release,
            Err(ReleaseError::NoCompatibleReleaseInstalled) => {
                let gh_release = github::find_matching_release(&version_req)?;
                Release::new(&mut cfg, gh_release.version().clone())
                    .map_err(Error::ReleaseLoadError)?
            }
            Err(err) => return Err(Error::ReleaseLoadError(err)),
        }
    };

    let asset = release.asset_remote().map_err(Error::ReleaseLoadError)?;

    if release.asset_need_fetched() {
        download_file(
            &release.get_asset().display().to_string(),
            &asset.as_ref(),
            release.get_asset(),
        )
        .map_err(Error::CannotUpdate)?;
        println!("**** asset downloaded");
    }

    release.asset_open().map_err(Error::ReleaseLoadError)?;

    if make_default {
        release
            .make_default(&cfg)
            .map_err(Error::ReleaseLoadError)?;
    }

    Ok(())
}

fn list() -> Result<(), Error> {
    Ok(())
}

fn remove() -> Result<(), Error> {
    Ok(())
}
