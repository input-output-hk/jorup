use crate::{channel::Channel, common::JorupConfig, release::Release};
use clap::ArgMatches;
use curl::easy::Easy;
use jorup_lib::{Version, VersionReq};

pub mod arg {
    use clap::{App, Arg, SubCommand};

    pub mod name {
        pub const COMMAND: &str = "update";
        pub const INIT: &str = "UPDATE_INIT";
        pub const CHANNEL_NAME: &str = "CHANNEL";
        pub const MAKE_DEFAULT: &str = "MAKE_DEFAULT";
    }

    pub fn command<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name(name::COMMAND)
            .about("sync and update the local channel")
            .arg(
                Arg::with_name(name::INIT)
                    .long("init")
                    .help("initialize do the initialization if not already the initialized (set default to latest stable)"),
            )
            .arg(
                Arg::with_name(name::CHANNEL_NAME)
                    .value_name(name::CHANNEL_NAME)
                    .help("update only this version, by default it will be all the installed channels")
                    .validator(validator::channel),
            )
            .arg(
                Arg::with_name(name::MAKE_DEFAULT)
                    .long("default")
                    .help("make the associated jormungandr release the default")
            )
    }

    mod validator {
        use std::str::FromStr as _;

        pub fn channel(arg: String) -> Result<(), String> {
            use crate::common::Channel;
            use error_chain::ChainedError as _;

            Channel::from_str(&arg)
                .map(|_channel| ())
                .map_err(|err| err.display_chain().to_string())
        }
    }
}

error_chain! {
    errors {
        Channel (channel: jorup_lib::Channel) {
            description("Error with the channel"),
            display("Error with channel: {}", channel),
        }

        Release (version: Version) {
            description("Error with the release"),
            display("Error with release: {}", version),
        }
    }
}

pub fn run<'a>(mut cfg: JorupConfig, matches: &ArgMatches<'a>) -> Result<()> {
    cfg.sync_jorfile().chain_err(|| {
        "Error while syncing releases and channels, no internet? try `--offline`..."
    })?;

    let make_default = matches.is_present(arg::name::MAKE_DEFAULT);

    let jor = cfg
        .load_jor()
        .chain_err(|| "No jorfile... cannot operate")?;

    let mut current_channel: crate::common::Channel = cfg.current_channel().clone();

    let entry = if let Some(channel) = matches.value_of(arg::name::CHANNEL_NAME) {
        // should be save to unwrap as we have set a validator in the Argument
        // for the CLI to check it is valid
        use crate::common::Channel::*;
        current_channel = channel.parse().unwrap();
        match &current_channel {
            Nightly => jor.search_entry(true, VersionReq::any()),
            Stable => jor.search_entry(false, VersionReq::any()),
            Specific { channel } => jor.entries().get(&channel),
        }
    } else {
        cfg.current_entry(&jor)
    };

    let entry = entry.ok_or(Error::from("channel does not exist"))?;

    dbg!(entry.channel());
    // prepare entry directory
    let channel = Channel::new(&cfg, entry.clone())
        .chain_err(|| ErrorKind::Channel(entry.channel().clone()))?;
    channel
        .prepare()
        .chain_err(|| ErrorKind::Channel(entry.channel().clone()))?;
    let release =
        if let Some(release) = jor.search_release(channel.entry().jormungandr_versions().clone()) {
            Release::new(&cfg, release.clone())
                .chain_err(|| ErrorKind::Release(release.version().clone()))?
        } else {
            bail!("No release")
        };
    let asset = release
        .asset_remote()
        .chain_err(|| ErrorKind::Release(release.version().clone()))?;

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
            println!("**** asset downloaded");
        }
    }

    release
        .asset_open()
        .chain_err(|| ErrorKind::Release(release.version().clone()))?;

    if make_default {
        release
            .make_default(&cfg)
            .chain_err(|| ErrorKind::Release(release.version().clone()))?;
        cfg.set_default_channel(current_channel)
            .chain_err(|| "cannot save default channel")?;
    }

    println!("**** channel updated to version {}", release.version());
    Ok(())
}
