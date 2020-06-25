use crate::{
    common::JorupConfig,
    utils::{
        blockchain::Blockchain,
        download::{self, Client},
        github,
        release::{list_installed_releases, Error as ReleaseError, Release},
        version::{Version, VersionReq},
    },
};
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
        #[structopt(short = "v", long = "version")]
        version_req: Option<VersionReq>,

        /// Install the latest version compatible with the specified blockchain
        #[structopt(short, long)]
        blockchain: Option<String>,

        /// Make the installed version default
        #[structopt(long)]
        make_default: bool,
    },
    /// List locally installed Jormungandr releases
    List,
    /// Remove the specified release
    Remove { version: Version },
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
    ReleaseLoad(#[source] ReleaseError),
    #[error("Cannot download and install an update")]
    CannotUpdate(#[source] download::Error),
    #[error("Error while listing releases")]
    ReleasesList(#[source] ReleaseError),
    #[error("Failed to remove a release")]
    RemoveRelease(#[source] std::io::Error),
    #[error("Failed to create the downloader client")]
    DownloaderCreate(#[source] download::Error),
    #[error("Error while creating directory: {1}")]
    CannotCreateDirectory(#[source] std::io::Error, std::path::PathBuf),
}

impl Command {
    pub fn run(self, cfg: JorupConfig) -> Result<(), Error> {
        match self {
            Command::Install {
                version_req,
                blockchain,
                make_default,
            } => install(cfg, version_req, blockchain, make_default),
            Command::List => list(cfg),
            Command::Remove { version } => remove(cfg, version),
        }
    }
}

fn install(
    mut cfg: JorupConfig,
    version_req: Option<VersionReq>,
    blockchain: Option<String>,
    make_default: bool,
) -> Result<(), Error> {
    if cfg.offline() {
        return Err(Error::Offline);
    }

    if version_req.is_some() && blockchain.is_some() {
        return Err(Error::MustNotSpecifyBlockchainAndVersion);
    }

    let load_latest = version_req.is_none() && blockchain.is_none();

    let version_req = match version_req {
        None => match blockchain {
            None => VersionReq::Latest,
            Some(blockchain_name) => Blockchain::load(&mut cfg, &blockchain_name)?
                .jormungandr_version_req()
                .clone(),
        },
        Some(version_req) => version_req,
    };

    let mut client = Client::new().map_err(Error::DownloaderCreate)?;

    let release = if load_latest {
        let gh_release =
            github::find_matching_release(&mut client, github::JORMUNGANDR, version_req)?;
        Release::new_unchecked(&cfg, gh_release.version().clone())
    } else {
        match Release::load(&cfg, &version_req) {
            Ok(release) => {
                if let Some(date) = release.version().get_nightly_date() {
                    if date < &chrono::Utc::now().date() {
                        let gh_release = github::find_matching_release(
                            &mut client,
                            github::JORMUNGANDR,
                            version_req,
                        )?;
                        Release::new_unchecked(&cfg, gh_release.version().clone())
                    } else {
                        release
                    }
                } else {
                    release
                }
            }
            Err(ReleaseError::NoCompatibleReleaseInstalled(_)) => {
                let gh_release =
                    github::find_matching_release(&mut client, github::JORMUNGANDR, version_req)?;
                Release::new_unchecked(&cfg, gh_release.version().clone())
            }
            Err(err) => return Err(Error::ReleaseLoad(err)),
        }
    };

    let asset = release
        .asset_remote(&mut client)
        .map_err(Error::ReleaseLoad)?;

    if release.asset_need_fetched() {
        std::fs::create_dir_all(release.dir())
            .map_err(|e| Error::CannotCreateDirectory(e, release.dir().clone()))?;
        client
            .download_file(
                &release.get_asset().display().to_string(),
                &asset.as_ref(),
                release.get_asset(),
            )
            .map_err(|e| {
                std::fs::remove_dir_all(release.dir()).expect("could not remove the release dir");
                Error::CannotUpdate(e)
            })?;
        println!("**** asset downloaded");
    }

    release.asset_open().map_err(Error::ReleaseLoad)?;

    if make_default {
        release.make_default(&cfg).map_err(Error::ReleaseLoad)?;
    }

    Ok(())
}

fn list(cfg: JorupConfig) -> Result<(), Error> {
    for release in list_installed_releases(&cfg).map_err(Error::ReleasesList)? {
        println!("{}", release.version());
    }
    Ok(())
}

fn remove(cfg: JorupConfig, version: Version) -> Result<(), Error> {
    let version_req = VersionReq::exact(version);
    let release = Release::load(&cfg, &version_req).map_err(Error::ReleaseLoad)?;
    std::fs::remove_dir_all(release.dir()).map_err(Error::RemoveRelease)?;

    Ok(())
}
