use curl::easy::Easy;
use indicatif::{ProgressBar, ProgressStyle};
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
    let progress = ProgressBar::new(100).with_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {msg} {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        );
    progress.set_message(&what);

    let mut handle = Easy::new();
    handle.url(url).unwrap();
    handle.progress(true).unwrap();
    let finalizer = progress.clone();
    handle
        .progress_function(move |total, so_far, _, _| {
            let total = total.floor() as u64;
            let so_far = so_far.floor() as u64;
            if total != 0 {
                progress.set_length(total);
                progress.set_position(so_far);
            }
            true
        })
        .unwrap();
    handle.follow_location(true).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(to.as_ref())
        .chain_err(|| ErrorKind::CannotCreateDestinationFile(to.as_ref().to_path_buf()))?;

    let res = {
        let mut transfers = handle.transfer();
        transfers
            .write_function(|data| {
                use std::io::Write as _;
                file.write_all(&data).unwrap();
                Ok(data.len())
            })
            .unwrap();
        transfers.perform().chain_err(|| {
            ErrorKind::CannotDownloadAsset(what.to_owned(), to.as_ref().to_path_buf())
        })
    };

    if res.is_err() {
        finalizer.finish_at_current_pos();
    } else {
        finalizer.finish_and_clear();
    }

    res
}
