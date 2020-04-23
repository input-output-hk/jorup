use super::download::download_to_writer;
use crate::utils::version::{SemVerError, Version, VersionReq};
use chrono::{offset::Utc, DateTime};
use serde::Deserialize;
use thiserror::Error;

pub struct Release {
    version: Version,
    assets: Vec<AssetDef>,
}

#[derive(Deserialize)]
struct ReleasesDef(Vec<ReleaseDef>);

#[derive(Deserialize)]
struct ReleaseDef {
    tag_name: String,
    assets: Vec<AssetDef>,
    published_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct AssetDef {
    #[serde(rename = "browser_download_url")]
    url: String,
    name: String,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to fetch releases")]
    CannotGetReleaseData(#[from] reqwest::Error),
    #[error("Cannot parse the release data")]
    MalformedReleaseData(#[from] serde_json::Error),
    #[error("No release matching {0}")]
    ReleaseNotFound(VersionReq),
}

fn download_release_by_url(url: &str) -> Result<ReleaseDef, Error> {
    let mut release_data_raw: Vec<u8> = Vec::new();
    download_to_writer("GitHub release", &url, &mut release_data_raw)?;
    serde_json::from_slice(&release_data_raw).map_err(Into::into)
}

fn get_exact_release(version: VersionReq) -> Result<Release, Error> {
    let version = version.into_version().unwrap();
    let url = format!(
        "https://api.github.com/repos/input-output-hk/jormungandr/releases/tags/{}",
        version.to_git_tag(),
    );
    let release_def = download_release_by_url(&url)?;
    Ok(Release {
        version,
        assets: release_def.assets,
    })
}

fn get_latest_release() -> Result<Release, Error> {
    let release_def = download_release_by_url(
        "https://api.github.com/repos/input-output-hk/jormungandr/releases/latest",
    )?;
    let version = Version::from_git_tag(&release_def.tag_name).unwrap();
    Ok(Release {
        version,
        assets: release_def.assets,
    })
}

fn get_nightly_release() -> Result<Release, Error> {
    let latest = get_latest_release()?;
    let release_def = download_release_by_url(
        "https://api.github.com/repos/input-output-hk/jormungandr/releases/tags/nightly",
    )?;
    let version = Version::from_git_tag(&release_def.tag_name)
        .unwrap()
        .configure_nightly(latest.version, release_def.published_at);
    Ok(Release {
        version,
        assets: release_def.assets,
    })
}

fn find_release_by_req(version_req: &VersionReq) -> Result<Release, Error> {
    let mut releases_data_raw: Vec<u8> = Vec::new();
    download_to_writer(
        "GitHub releases",
        "https://api.github.com/repos/input-output-hk/jormungandr/releases",
        &mut releases_data_raw,
    )?;

    let releases: ReleasesDef = serde_json::from_slice(&releases_data_raw)?;

    let release = releases
        .0
        .into_iter()
        .map(|release_def| {
            Ok::<_, SemVerError>(Release {
                version: Version::from_git_tag(&release_def.tag_name)?,
                assets: release_def.assets,
            })
        })
        .filter_map(core::result::Result::ok)
        .find(|release| version_req.matches(&release.version));

    match release {
        Some(release) => Ok(release),
        None => Err(Error::ReleaseNotFound(version_req.clone())),
    }
}

pub fn find_matching_release(version_req: VersionReq) -> Result<Release, Error> {
    match version_req {
        VersionReq::Latest => get_latest_release(),
        VersionReq::Nightly => get_nightly_release(),
        VersionReq::Stable(_) => find_release_by_req(&version_req),
        VersionReq::ExactStable(_) => get_exact_release(version_req),
    }
}

impl Release {
    pub fn get_asset_url(&self, platform: &str) -> Option<&str> {
        let ext = if platform.contains("windows") {
            "zip"
        } else {
            "tar.gz"
        };
        let expected_name = format!(
            "jormungandr-{}-{}-generic.{}",
            self.version.to_version_number(),
            platform,
            ext
        );
        let maybe_asset = self.assets.iter().find(|asset| asset.name == expected_name);
        maybe_asset.map(|asset| &asset.url[..])
    }

    pub fn version(&self) -> &Version {
        &self.version
    }
}
