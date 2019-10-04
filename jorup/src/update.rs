use crate::common::JorupConfig;
use clap::ArgMatches;
use curl::easy::Easy;
use semver::Version;
use std::path::PathBuf;

pub mod arg {
    use clap::{App, Arg, SubCommand};

    pub mod name {
        pub const COMMAND: &str = "update";
        pub const INIT: &str = "UPDATE_INIT";
        pub const CHANNEL_NAME: &str = "CHANNEL";
    }

    pub fn command<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name(name::COMMAND)
            .about("sync and update the local channel")
            .arg(
                Arg::with_name(name::INIT)
                    .long("init")
                    .help("initialise do the initialization if not already the initialized (set default to latest stable)"),
            )
            .arg(
                Arg::with_name(name::CHANNEL_NAME)
                    .value_name(name::CHANNEL_NAME)
                    .help("update only this version, by default it will be all the installed channels")
                    .validator(validator::channel),
            )
    }

    mod validator {
        use std::str::FromStr as _;

        pub fn channel(arg: String) -> Result<(), String> {
            use error_chain::ChainedError as _;
            use jorup_lib::Channel;

            Channel::from_str(&arg)
                .map(|_channel| ())
                .map_err(|err| err.display_chain().to_string())
        }
    }
}

error_chain! {}

pub fn run<'a>(cfg: JorupConfig, matches: &ArgMatches<'a>) -> Result<()> {
    cfg.sync_jorfile().chain_err(|| {
        "Error while syncing releases and channels, no internet? try `--offline`..."
    })?;

    let jor = cfg
        .load_jor()
        .chain_err(|| "No jorfile... cannot operate")?;

    let channel = if let Some(channel) = matches.value_of(arg::name::CHANNEL_NAME) {
        // should be save to unwrap as we have set a validator in the Argument
        // for the CLI to check it is valid
        channel.parse().unwrap()
    } else {
        let mut channel_list = jor.entries().keys();
        #[cfg(nightly)]
        debug_assert!(channel_list.is_sorted());
        if let Some(channel) = channel_list.next() {
            channel.clone()
        } else {
            bail!("No channels available in the jorfile, this may happen if you are running jorup for the first time and running with `--offline`. Try without `--offline`")
        }
    };

    if let Some(entry) = jor.search_entry(
        channel.is_nightly(),
        semver::VersionReq::exact(channel.version()),
    ) {
        // prepare entry directory
        let channel = Channel::new(&cfg, entry.clone())?;
        let release = if let Some(release) =
            jor.search_release(channel.entry().jormungandr_versions().clone())
        {
            Release::new(&cfg, release.clone())?
        } else {
            bail!("No release")
        };
        let asset = release.asset_remote()?;

        if release.asset_need_fetched() && !cfg.offline() {
            let progress = indicatif::ProgressBar::new(100).with_style(
            indicatif::ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {msg} {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        );
            progress.set_message(&release.get_asset().display().to_string());

            let mut handle = Easy::new();
            handle.url(asset.as_ref()).unwrap();
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
                .open(release.get_asset())
                .chain_err(|| format!("cannot create file `{}`", release.get_asset().display()))?;

            let res = {
                let mut transfers = handle.transfer();
                transfers
                    .write_function(|data| {
                        use std::io::Write as _;
                        file.write_all(&data).unwrap();
                        Ok(data.len())
                    })
                    .unwrap();
                transfers.perform()
            };

            if let Err(err) = res {
                finalizer.finish_at_current_pos();
                eprintln!("cannot download asset: {}", err);
            } else {
                finalizer.finish_and_clear();
                println!("asset downloaded");
            }
        }

        release.asset_open()?;

        release.make_default(&cfg)?;

        println!("**** channel updated to version {}", release.version());
        Ok(())
    } else {
        bail!("channel doest not exist")
    }
}

pub struct Channel {
    entry: jorup_lib::Entry,

    path: PathBuf,
}

pub struct Release {
    release: jorup_lib::Release,

    path: PathBuf,
}

impl Channel {
    pub fn new(cfg: &JorupConfig, entry: jorup_lib::Entry) -> Result<Self> {
        let path = cfg
            .channel_dir()
            .join(entry.channel().channel())
            .join(entry.channel().version().to_string());
        let path = if let Some(date) = entry.channel().nightly_date() {
            path.join(date)
        } else {
            path
        };
        std::fs::create_dir_all(&path)
            .chain_err(|| format!("Error while creating directory '{}'", path.display()))?;
        Ok(Channel { entry, path })
    }

    pub fn entry(&self) -> &jorup_lib::Entry {
        &self.entry
    }

    pub fn dir(&self) -> &PathBuf {
        &self.path
    }
}

impl Release {
    pub fn new(cfg: &JorupConfig, release: jorup_lib::Release) -> Result<Self> {
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
