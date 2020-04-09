use super::download::download_to_reader;
use semver::{SemVerError, Version, VersionReq};
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

pub fn find_matching_release(version_req: &VersionReq) -> Result<Release, Error> {
    let mut releases_data_raw: Vec<u8> = Vec::new();
    download_to_reader(
        "GitHub releases",
        "https://api.github.com/repos/input-output-hk/jormungandr/releases",
        &mut releases_data_raw,
    )?;

    let releases: ReleasesDef = serde_json::from_slice(&releases_data_raw)?;

    let release = releases
        .0
        .into_iter()
        .map(|release_def| {
            let (_, semver_str) = release_def.tag_name[..].split_at(1);
            Ok::<_, SemVerError>(Release {
                version: Version::parse(semver_str)?,
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

impl Release {
    pub fn get_asset_url(&self, platform: &str) -> Option<&str> {
        let ext = if platform.contains("windows") {
            "zip"
        } else {
            "tar.gz"
        };
        let expected_name = format!(
            "jormungandr-v{}-{}-generic.{}",
            self.version.to_string(),
            platform,
            ext
        );
        println!("{}", expected_name);
        let maybe_asset = self.assets.iter().find(|asset| asset.name == expected_name);
        maybe_asset.map(|asset| &asset.url[..])
    }

    pub fn version(&self) -> &Version {
        &self.version
    }
}
