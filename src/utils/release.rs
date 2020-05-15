use crate::{
    common::JorupConfig,
    utils::{
        download::Client,
        github,
        version::{Version, VersionReq},
    },
};
use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

const TARGET: &str = env!("TARGET");

pub struct Release {
    version: Version,
    path: PathBuf,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot read the release directory: {1}")]
    ReleaseDirectory(#[source] io::Error, PathBuf),
    #[error("No compatible release installed, expecting {0}")]
    NoCompatibleReleaseInstalled(VersionReq),
    #[error(transparent)]
    GitHub(#[from] crate::utils::github::Error),
    #[error("Error while opening file: {1}")]
    CannotOpenFile(#[source] io::Error, PathBuf),
    #[error("Asset not found for the current platform")]
    AssetNotFound,
    #[cfg(unix)]
    #[error("Cannot unpack assset: {1}")]
    CannotUnpack(#[source] io::Error, PathBuf),
    #[cfg(windows)]
    #[error("Cannot unpack assset: {1}")]
    CannotUnpack(#[source] zip::result::ZipError, PathBuf),
}

pub fn list_installed_releases(cfg: &JorupConfig) -> Result<impl Iterator<Item = Version>, Error> {
    Ok(fs::read_dir(cfg.release_dir())
        .map_err(|err| Error::ReleaseDirectory(err, cfg.release_dir()))?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .map(|etype| etype.is_dir())
                .unwrap_or_else(|_| false)
        })
        .filter_map(|entry| {
            entry
                .file_name()
                .as_os_str()
                .to_str()
                .map(|name| Version::parse(name))
                .and_then(Result::ok)
        }))
}

impl Release {
    /// load the latest locally installed release
    pub fn load(cfg: &mut JorupConfig, version_req: &VersionReq) -> Result<Self, Error> {
        let version = list_installed_releases(cfg)?
            .filter(|version| version_req.matches(version))
            .max()
            .ok_or_else(|| Error::NoCompatibleReleaseInstalled(version_req.clone()))?;
        let path = cfg.release_dir().join(version.to_string());
        Ok(Release { version, path })
    }

    /// load a potentially not installed release
    pub fn new_unchecked(cfg: &JorupConfig, version: Version) -> Self {
        let path = cfg.release_dir().join(version.to_string());
        Release { version, path }
    }

    pub fn make_default(&self, cfg: &JorupConfig) -> Result<(), Error> {
        let bin_dir = cfg.bin_dir();

        let install_jormungandr = bin_dir.join("jormungandr");
        let install_jcli = bin_dir.join("jcli");

        create_symlink(self.get_jormungandr(), install_jormungandr).unwrap();
        create_symlink(self.get_jcli(), install_jcli).unwrap();

        Ok(())
    }

    pub fn get_jormungandr(&self) -> PathBuf {
        self.dir().join("jormungandr")
    }

    pub fn get_jcli(&self) -> PathBuf {
        self.dir().join("jcli")
    }

    #[cfg(windows)]
    pub fn get_asset(&self) -> PathBuf {
        self.dir().join("archive.zip")
    }

    #[cfg(unix)]
    pub fn get_asset(&self) -> PathBuf {
        self.dir().join("archive.tar.gz")
    }

    pub fn asset_need_fetched(&self) -> bool {
        !self.get_asset().is_file()
    }

    pub fn asset_need_open(&self) -> bool {
        !self.get_jormungandr().is_file() || !self.get_jcli().is_file()
    }

    pub fn asset_open(&self) -> Result<(), Error> {
        if !self.asset_need_open() {
            return Ok(());
        }
        let file =
            File::open(self.get_asset()).map_err(|e| Error::CannotOpenFile(e, self.get_asset()))?;
        self.unpack_asset(file)
    }

    #[cfg(windows)]
    fn unpack_asset(&self, file: File) -> Result<(), Error> {
        let mut archive = zip::read::ZipArchive::new(file)
            .map_err(|e| Error::CannotUnpack(e, self.get_asset()))?;
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| Error::CannotUnpack(e, self.get_asset()))?;
            let path = self.dir().join(file.name());
            let mut decompressed_file = File::create(path.clone())
                .map_err(|e| Error::CannotOpenFile(e, path.to_path_buf()))?;
            std::io::copy(&mut file, &mut decompressed_file)
                .map_err(|e| Error::CannotOpenFile(e, path.to_path_buf()))?;
        }

        Ok(())
    }

    #[cfg(unix)]
    fn unpack_asset(&self, file: File) -> Result<(), Error> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let content = GzDecoder::new(file);
        let mut archive = Archive::new(content);
        archive.set_preserve_permissions(true);
        archive
            .unpack(self.dir())
            .map_err(|e| Error::CannotUnpack(e, self.get_asset()))?;

        Ok(())
    }

    pub fn asset_remote(&self, client: &mut Client) -> Result<String, Error> {
        let release =
            github::find_matching_release(client, VersionReq::exact(self.version.clone()))?;
        match release.get_asset_url(TARGET) {
            Some(url) => Ok(url.to_string()),
            None => Err(Error::AssetNotFound),
        }
    }

    pub fn dir(&self) -> &PathBuf {
        &self.path
    }
}

#[cfg(unix)]
fn create_symlink<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn create_symlink<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> io::Result<()> {
    std::os::windows::fs::symlink_file(src, dst)
}
