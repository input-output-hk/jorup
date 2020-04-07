pub mod arg;
use crate::common::HelConfig;
use clap::ArgMatches;
use dialoguer::{Input, Validator};
use jorup_lib::{
    ChannelError, ChannelErrorKind, Disposition, EntryBuilder, Genesis, PartialChannelDesc,
};
use semver::VersionReq;
use std::process::Stdio;

#[derive(Clone, Copy)]
struct VersionReqValidator;
impl Validator for VersionReqValidator {
    type Err = semver::ReqParseError;
    fn validate(&self, text: &str) -> std::result::Result<(), Self::Err> {
        text.parse::<VersionReq>().map(|_| ())
    }
}

error_chain! {
    foreign_links {
        Io(std::io::Error);
    }

    links {
        InvalidChannel(ChannelError, ChannelErrorKind);
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

    let channel: PartialChannelDesc = matches.value_of(arg::name::CHANNEL_NAME).unwrap().parse()?;
    let channel = channel.into_channel_desc();

    if jor.entries().contains_key(&channel) {
        bail!("channel already exist")
    }

    let mut entry_builder = EntryBuilder::default();

    entry_builder.channel(channel);

    // 1. check if the version requirement was given from the command line parameters
    //    otherwise ask for it
    let version_req = if let Some(req) = matches.value_of(arg::name::VERSION_REQ) {
        req.parse::<VersionReq>().unwrap()
    } else {
        Input::new()
            .with_prompt("jormungandr supported version")
            .validate_with(VersionReqValidator)
            .interact()?
    };
    entry_builder.jormungandr_versions(version_req.clone());

    // 2. check if the description was given from the command line parameters
    //    otherwise ask for it
    let description = if let Some(description) = matches.value_of(arg::name::DESCRIPTION) {
        description.to_owned()
    } else {
        Input::new()
            .with_prompt("description")
            .default(format!("testnet for jormungandr version {}", version_req))
            .interact()?
    };
    entry_builder.description(description);

    let disposition = match matches.value_of(arg::name::DISPOSITION).unwrap() {
        "up" => Disposition::Up,
        "down" => Disposition::Down,
        _ => unreachable!("This value is already covered by validating possible inputs"),
    };
    entry_builder.disposition(disposition);

    let genesis_data = if let Some(genesis_file) = matches.value_of(arg::name::GENESIS_FILE) {
        use std::io::Read as _;
        let mut content = String::new();
        std::fs::File::open(genesis_file)?.read_to_string(&mut content)?;
        content
    } else {
        let mut jcli = cfg.jcli(&version_req).unwrap();
        let output = String::from_utf8(
            jcli.args(&["genesis", "init"])
                .stdout(Stdio::piped())
                .output()?
                .stdout,
        )
        .unwrap();
        if let Some(content) = dialoguer::Editor::new().edit(&output)? {
            content
        } else {
            panic!("needed to save the file in order to validate the data")
        }
    };

    let mut jcli = cfg.jcli(&version_req).unwrap();
    let mut child = jcli
        .args(&["genesis", "encode"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");
    {
        use std::io::Write as _;
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(genesis_data.as_bytes())?;
    }
    let block0 = child
        .wait_with_output()
        .expect("failed to read stdout")
        .stdout;

    let mut jcli = cfg.jcli(&version_req).unwrap();
    let mut child = jcli
        .args(&["genesis", "hash"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");
    {
        use std::io::Write as _;
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(block0.as_slice())?;
    }
    let block0_hash = String::from_utf8(
        child
            .wait_with_output()
            .expect("failed to read stdout")
            .stdout,
    )
    .unwrap();

    let genesis = Genesis {
        block0_hash: block0_hash.trim_end().to_owned(),
    };

    entry_builder.genesis(genesis);

    let entry = entry_builder.build();

    jor.add_entry(entry)
        .chain_err(|| ErrorKind::CannotUpdateRelease)?;

    cfg.save_release_file(jor)
        .chain_err(|| "error while saving the new entry to release file")
}
