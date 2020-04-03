use jorup_lib::download;
use serde::Deserialize;

error_chain! {
    errors {
        CannotGetReleaseData(version: String) {
            description("failed to get any data for the requested release")
            display("failed to get any data for version v{}", version)
        }

        MalformedReleaseData {
            description("cannot parse the release data")
        }

        AssetNotFound(version: String, platform: String) {
            description("asset not found for the requested version and platform")
            display("no assets with version v{} for {} found", version, platform)
        }
    }
}

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

/// Get the URL to download an asset. Version should specified as `x.y.z`.
/// `None` tells to download the latest version.
pub fn get_asset_url(version: Option<&str>, platform: &str) -> Result<String> {
    let release_url = match version {
        Some(version) => format!(
            "https://api.github.com/repos/input-output-hk/jormungandr/releases/tags/v{}",
            version
        ),
        None => {
            "https://api.github.com/repos/input-output-hk/jormungandr/releases/latest".to_owned()
        }
    };

    let mut release_data_raw: Vec<u8> = Vec::new();
    download(
        version.unwrap_or("latest"),
        &release_url,
        &mut release_data_raw,
    )
    .chain_err(|| ErrorKind::CannotGetReleaseData(version.unwrap_or("latest").to_owned()))?;
    let release: ReleaseDef =
        serde_json::from_slice(&release_data_raw).chain_err(|| ErrorKind::MalformedReleaseData)?;

    let ext = if platform.contains("windows") {
        "zip"
    } else {
        "tar.gz"
    };

    let expected_name = format!("jormungandr-{}-{}.{}", release.tag_name, platform, ext);

    let maybe_asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name == expected_name);

    match maybe_asset {
        Some(asset) => Ok(asset.url),
        None => Err(Error::from_kind(ErrorKind::AssetNotFound(
            version.unwrap_or("latest").to_owned(),
            platform.to_owned(),
        ))),
    }
}
