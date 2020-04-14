use crate::common::JorupConfig;
use crate::utils::github;
use semver::VersionReq;
use std::{fs::File, io, path::PathBuf};
use thiserror::Error;

const TARGET: &str = env!("TARGET");

pub struct Release {
    release: github::Release,

    path: PathBuf,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    GitHub(#[from] crate::utils::github::Error),
    #[error("Error while creating directory: {1}")]
    CannotCreateDirectory(#[source] io::Error, PathBuf),
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

impl Release {
    pub fn new(cfg: &mut JorupConfig, req: &VersionReq) -> Result<Self, Error> {
        let release = github::find_matching_release(req)?;

        let path = cfg.release_dir().join(release.version().to_string());
        std::fs::create_dir_all(&path)
            .map_err(|e| Error::CannotCreateDirectory(e, path.clone()))?;
        Ok(Release { release, path })
    }

    pub fn make_default(&self, cfg: &JorupConfig) -> Result<(), Error> {
        let bin_dir = cfg.bin_dir();

        let install_jormungandr = bin_dir.join("jormungandr");
        let install_jcli = bin_dir.join("jcli");

        std::fs::copy(self.get_jormungandr(), install_jormungandr).unwrap();
        std::fs::copy(self.get_jcli(), install_jcli).unwrap();

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

    pub fn asset_remote(&self) -> Result<&str, Error> {
        match self.release.get_asset_url(TARGET) {
            Some(url) => Ok(url),
            None => Err(Error::AssetNotFound),
        }
    }

    pub fn version(&self) -> &semver::Version {
        self.release.version()
    }

    pub fn dir(&self) -> &PathBuf {
        &self.path
    }
}
