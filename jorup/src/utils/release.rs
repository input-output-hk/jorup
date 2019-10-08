use crate::common::JorupConfig;
use jorup_lib::{Version, VersionReq};
use std::path::PathBuf;

error_chain! {}

pub struct Release {
    release: jorup_lib::Release,

    path: PathBuf,
}

impl Release {
    pub fn new(cfg: &mut JorupConfig, req: &VersionReq) -> Result<Self> {
        let release = cfg
            .load_jor()
            .unwrap()
            .search_release(req.clone())
            .map(|c| c.clone())
            .ok_or_else(|| format!("No release that matches `{}`", req))?;

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
        self.dir().join("archive.tar.bz2")
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
        use flate2::read::GzDecoder;
        use std::fs::File;
        use tar::Archive;

        let file = File::open(self.get_asset())
            .chain_err(|| format!("Cannot open `{}`", self.get_asset().display()))?;
        let content = GzDecoder::new(file);
        let mut archive = Archive::new(content);
        archive.set_preserve_permissions(true);
        archive
            .unpack(self.dir())
            .chain_err(|| format!("cannot unpack `{}`", self.get_asset().display()))?;

        Ok(())
    }

    pub fn asset_remote(&self) -> Result<&jorup_lib::Url> {
        if let Some(platform) = platforms::guess_current() {
            if let Some(asset) = self.release.assets().get(platform.target_triple) {
                Ok(asset)
            } else {
                bail!(format!("No assets for host `{}`", platform.target_triple))
            }
        } else {
            bail!("cannot guess host system")
        }
    }

    pub fn version(&self) -> &Version {
        self.release.version()
    }

    pub fn dir(&self) -> &PathBuf {
        &self.path
    }
}
