use indicatif::{ProgressBar, ProgressStyle};
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

const INDICATIF_TEMPLATE: &'static str =
    "[{elapsed_precise}] [{bar:40.cyan/blue}] {msg} {bytes}/{total_bytes} ({bytes_per_sec}, {eta})";
const INDICATIF_LENGTH: u64 = 100;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot create destination file for download: {0}")]
    CannotCreateDestinationFile(#[source] io::Error, PathBuf),
    #[error("Failed to download '{asset}' into file {destination}")]
    CannotDownloadAsset {
        #[source]
        source: reqwest::Error,
        asset: String,
        destination: PathBuf,
    },
}

struct WriterWithProgress<'a, W> {
    inner: W,
    progress: &'a ProgressBar,
    written: u64,
}

impl<'a, W> io::Write for WriterWithProgress<'a, W>
where
    W: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write_all(&buf)?;
        self.written = self.written + buf.len() as u64;
        self.progress.set_position(self.written);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub fn download_to_writer<W: io::Write>(
    what: &str,
    url: &str,
    to: &mut W,
) -> std::result::Result<(), reqwest::Error> {
    let style = ProgressStyle::default_bar().template(INDICATIF_TEMPLATE);
    let progress = ProgressBar::new(INDICATIF_LENGTH).with_style(style);
    progress.set_message(what);

    let res = download_internal(url, to, &progress);

    if res.is_err() {
        progress.finish_at_current_pos();
    } else {
        progress.finish_and_clear();
    }

    res
}

fn download_internal<W: io::Write>(
    url: &str,
    to: &mut W,
    progress: &ProgressBar,
) -> std::result::Result<(), reqwest::Error> {
    let client = reqwest::blocking::ClientBuilder::new()
        .gzip(true)
        .user_agent(APP_USER_AGENT)
        .build()?;
    let mut response = client.execute(client.get(url).build()?)?;
    if let Some(total) = response.content_length() {
        progress.set_length(total);
        let mut writer = WriterWithProgress {
            inner: to,
            progress,
            written: 0,
        };
        response.copy_to(&mut writer)
    } else {
        response.copy_to(to)
    }
    .map(|_| ())
}

pub fn download_file<P: AsRef<Path>>(what: &str, url: &str, to: P) -> Result<(), Error> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(to.as_ref())
        .map_err(|e| Error::CannotCreateDestinationFile(e, to.as_ref().to_path_buf()))?;

    download_to_writer(what, url, &mut file).map_err(|e| Error::CannotDownloadAsset {
        source: e,
        asset: what.to_owned(),
        destination: to.as_ref().to_path_buf(),
    })
}
