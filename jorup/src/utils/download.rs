use jorup_lib::download as lib_download;
use std::path::{Path, PathBuf};

error_chain! {
    errors {
        CannotCreateDestinationFile(path: PathBuf) {
            description("Cannot create file"),
            display("Cannot create destination file for download: {}", path.display()),
        }

        CannotDownloadAsset(asset: String, into: PathBuf) {
            description("Failed to download asset"),
            display("Failed to download '{}' into file {}", asset, into.display())
        }
    }
}

pub fn download<P: AsRef<Path>>(what: &str, url: &str, to: P) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(to.as_ref())
        .chain_err(|| ErrorKind::CannotCreateDestinationFile(to.as_ref().to_path_buf()))?;

    lib_download(what, url, &mut file)
        .chain_err(|| ErrorKind::CannotDownloadAsset(what.to_owned(), to.as_ref().to_path_buf()))
}
