pub mod arg;
use crate::common::HelConfig;
use clap::ArgMatches;
use curl::easy::Easy;
use jorup_lib::{ReleaseBuilder, UrlBuilder, AVAILABLE_PLATFORMS};

error_chain! {
    foreign_links {
        Io(std::io::Error);
        SemVer(semver::SemVerError);
    }

    errors {
        CannotOpenReleaseFile {
            description("error while loading the release file")
        }

        CannotUpdateRelease {
            description("cannot add entry in the release file")
        }
    }
}

pub fn run<'a>(cfg: HelConfig, matches: &ArgMatches<'a>) -> Result<()> {
    match matches.subcommand() {
        (arg::name::COMMAND_ADD, Some(matches)) => run_add(cfg, matches),
        (arg::name::COMMAND_RM, Some(matches)) => run_rm(cfg, matches),
        (_, _) => {
            eprintln!("{}", matches.usage());
            Ok(())
        }
    }
}

fn run_add<'a>(cfg: HelConfig, matches: &ArgMatches<'a>) -> Result<()> {
    let mut jor = cfg
        .load_release_file()
        .chain_err(|| ErrorKind::CannotOpenReleaseFile)?;

    let version = matches.value_of(arg::name::RELEASE_NAME).unwrap().parse()?;

    if jor.releases().contains_key(&version) {
        bail!("version already exist")
    }

    let mut release_builder = ReleaseBuilder::default();

    release_builder.version(version.clone());

    let mut url_builder = UrlBuilder::default();
    url_builder
        .root("https://github.com/input-output-hk")
        .version(version.clone());
    for platform in AVAILABLE_PLATFORMS.iter() {
        let url = url_builder.clone().platform(platform.clone()).build();

        let progress = indicatif::ProgressBar::new(100).with_style(
            indicatif::ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {msg} {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        );
        let target = format!("jormungandr-v{}-{}", version, platform.target_triple);
        progress.set_message(&target);

        let mut handle = Easy::new();
        handle.url(url.as_ref()).unwrap();
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
        let mut bytes = Vec::new();

        let res = {
            let mut transfers = handle.transfer();
            transfers
                .write_function(|data| {
                    bytes.extend_from_slice(&data);
                    Ok(data.len())
                })
                .unwrap();
            transfers.perform()
        };

        if let Err(err) = res {
            finalizer.finish_at_current_pos();
            eprintln!("{}", err);
        } else if !bytes.starts_with(b"Not Found") {
            release_builder.add_assets(platform.target_triple, url);
            finalizer.finish_and_clear();
            println!("'{}' added to the release's assets", target);
        } else {
            finalizer.finish_and_clear();
        }
    }

    let release = release_builder.build();

    jor.add_release(release)
        .chain_err(|| ErrorKind::CannotUpdateRelease)?;

    cfg.save_release_file(jor)
        .chain_err(|| "error while saving the new entry to release file")
}

fn run_rm<'a>(cfg: HelConfig, matches: &ArgMatches<'a>) -> Result<()> {
    let mut jor = cfg
        .load_release_file()
        .chain_err(|| ErrorKind::CannotOpenReleaseFile)?;

    let version = matches.value_of(arg::name::RELEASE_NAME).unwrap().parse()?;

    if !jor.releases().contains_key(&version) {
        bail!("version does not exist")
    }

    jor.remove_release(&version);

    cfg.save_release_file(jor)
        .chain_err(|| "error while saving the new entry to release file")
}
