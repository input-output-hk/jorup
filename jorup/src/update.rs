use crate::{common::JorupConfig, utils::channel::Channel, utils::release::Release};
use clap::ArgMatches;
use curl::easy::Easy;
use jorup_lib::Version;

pub mod arg {
    use crate::utils::channel::Channel;
    use clap::{App, Arg, SubCommand};

    pub mod name {
        pub const COMMAND: &str = "update";
        pub const MAKE_DEFAULT: &str = "MAKE_DEFAULT";
    }

    pub fn command<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name(name::COMMAND)
            .about("sync and update the local channel")
            .arg(Channel::arg())
            .arg(
                Arg::with_name(name::MAKE_DEFAULT)
                    .long("default")
                    .help("make the associated jormungandr release the default"),
            )
    }
}

error_chain! {
    errors {
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

    // prepare entry directory
    let channel = Channel::load(&mut cfg, matches)
        .chain_err(|| "Cannot run the node without valid channel")?;
    channel
        .prepare()
        .chain_err(|| "Cannot run the node without valid channel")?;
    let release = Release::new(&mut cfg, channel.jormungandr_version_req())
        .chain_err(|| "Cannot run without compatible release")?;
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
        cfg.set_default_channel(channel.channel_version().clone())
            .chain_err(|| "cannot save default channel")?;
    }

    println!(
        "**** channel {} updated to version {}",
        channel.channel_version(),
        channel.entry().channel()
    );
    println!("**** jormungandr updated to version {}", release.version());
    Ok(())
}
