use crate::common::JorupConfig;
use crate::utils::github;
use semver::VersionReq;
use std::{fs::File, path::PathBuf};

error_chain! {
    links {
        GitHub(github::Error, github::ErrorKind);
    }
}

pub struct Release {
    release: github::Release,

    path: PathBuf,
}

impl Release {
    pub fn new(cfg: &mut JorupConfig, req: &VersionReq) -> Result<Self> {
        let release = github::find_matching_release(req)?;

        let path = cfg.release_dir().join(release.version().to_string());
        std::fs::create_dir_all(&path)
            .chain_err(|| format!("Error while creating directory '{}'", path.display()))?;
        Ok(Release { release, path })
    }

    pub fn make_default(&self, cfg: &JorupConfig) -> Result<()> {
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

    pub fn asset_open(&self) -> Result<()> {
        if !self.asset_need_open() {
            return Ok(());
        }
        let file = File::open(self.get_asset())
            .chain_err(|| format!("Cannot open `{}`", self.get_asset().display()))?;
        self.unpack_asset(file)
    }

    #[cfg(windows)]
    fn unpack_asset(&self, file: File) -> Result<()> {
        let mut archive = zip::read::ZipArchive::new(file)
            .chain_err(|| format!("cannot unpack `{}`", self.get_asset().display()))?;
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .chain_err(|| "cannot get the next file from the archive")?;
            let path = self.dir().join(file.name());
            let mut decompressed_file = File::create(path.clone())
                .chain_err(|| format!("cannot write to {}", path.as_path().display()))?;
            std::io::copy(&mut file, &mut decompressed_file)
                .chain_err(|| format!("cannot write to {}", path.as_path().display()))?;
        }

        Ok(())
    }

    #[cfg(unix)]
    fn unpack_asset(&self, file: File) -> Result<()> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let content = GzDecoder::new(file);
        let mut archive = Archive::new(content);
        archive.set_preserve_permissions(true);
        archive
            .unpack(self.dir())
            .chain_err(|| format!("cannot unpack `{}`", self.get_asset().display()))?;

        Ok(())
    }

    pub fn asset_remote(&self) -> Result<&str> {
        if let Some(platform) = platforms::guess_current() {
            match self.release.get_asset_url(platform.target_triple) {
                Some(url) => Ok(url),
                None => bail!("asset not found for the current platform"),
            }
        } else {
            bail!("cannot guess host system")
        }
    }

    pub fn version(&self) -> &semver::Version {
        self.release.version()
    }

    pub fn dir(&self) -> &PathBuf {
        &self.path
    }
}
