use crate::{
    common::JorupConfig,
    utils::{blockchain::Blockchain, release::Release, runner::RunnerControl, version::VersionReq},
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};
use structopt::StructOpt;
use thiserror::Error;

/// Run the jormungandr
#[derive(Debug, StructOpt)]
pub struct Command {
    /// The blockchain to run jormungandr for
    blockchain: String,

    /// The version of Jormungandr to run. If not specified, the latest
    /// compatible version will be used.
    #[structopt(short = "v", long = "version")]
    version_req: Option<VersionReq>,

    /// Run the node as a daemon
    #[structopt(short, long)]
    daemon: bool,

    /// Provide a custom configuration file to the node.
    ///
    /// Note that when using this flag `jorup` will not provide any
    /// configuration to `jormungandr` besides what you specify in the
    /// configuration file and extra arguments. The default configuration values
    /// can be obtained with `jorup defaults`.
    #[structopt(long)]
    config: Option<PathBuf>,

    /// The REST API address to listen
    ///
    /// When provided, this will be forwared to to jormungandr as a command line
    /// argument.
    #[structopt(long)]
    rest_listen: Option<SocketAddr>,

    /// The directory containing jormungandr and jcli, can be useful for
    /// development purposes. When provided, the `--version` flag is ignored.
    #[structopt(long)]
    bin: Option<PathBuf>,

    /// Extra parameters to pass on to the node
    ///
    /// Add pass on extra parameters to jormungandr for example, this command
    /// allows to change the default REST listen address, or to use a specific
    /// log formatting or output.
    extra: Vec<String>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot run the node without valid blockchain")]
    NoValidBlockchain(#[source] crate::utils::blockchain::Error),
    #[error("Cannot run without compatible release")]
    NoCompatibleRelease(#[source] crate::utils::release::Error),
    #[error("No binaries for this blockchain")]
    NoCompatibleBinaries,
    #[error("Unable to start the runner controller")]
    CannotStartRunnerController(#[source] crate::utils::runner::Error),
    #[error("Unable to start node")]
    Start(#[source] crate::utils::runner::Error),
    #[error("Cannot transform the configuration file path to its canonical form")]
    Canonicalize(#[source] std::io::Error),
    #[error("cannot read jormungandr configuration file")]
    Config(#[source] crate::jormungandr_config::Error),
}

impl Command {
    pub fn run(self, mut cfg: JorupConfig) -> Result<(), Error> {
        // prepare entry directory
        let blockchain =
            Blockchain::load(&mut cfg, &self.blockchain).map_err(Error::NoValidBlockchain)?;
        blockchain.prepare().map_err(Error::NoValidBlockchain)?;

        let bin = if let Some(dir) = self.bin {
            eprintln!("WARN: using custom binaries from {}", dir.display());
            std::fs::canonicalize(dir).map_err(Error::Canonicalize)?
        } else {
            let release = if let Some(version_req) = self.version_req {
                Release::load(&cfg, &version_req)
            } else {
                Release::load(&cfg, blockchain.jormungandr_version_req())
            }
            .map_err(|err| {
                eprintln!("HINT: run `jorup node install`");
                Error::NoCompatibleRelease(err)
            })?;

            if release.asset_need_fetched() {
                // asset release is not available
                return Err(Error::NoCompatibleBinaries);
            }

            release.dir().clone()
        };

        let mut runner =
            RunnerControl::new(&blockchain, bin).map_err(Error::CannotStartRunnerController)?;

        let default_config = self.config.is_none();
        let extra = {
            let mut extra = self.extra;
            if let Some(config_path) = &self.config {
                let config_path =
                    std::fs::canonicalize(config_path).map_err(Error::Canonicalize)?;
                extra.extend_from_slice(&[
                    "--config".to_string(),
                    config_path.display().to_string(),
                ]);
            }
            extra
        };

        let rest_addr = match self.rest_listen {
            Some(addr) => Some(addr),
            None => match &self.config {
                Some(config) => crate::jormungandr_config::load_config(config)
                    .map(|config| config.rest.map(|rest| rest.listen))
                    .map_err(Error::Config)?,
                None => {
                    if default_config {
                        Some(SocketAddr::new(
                            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                            8080,
                        ))
                    } else {
                        None
                    }
                }
            },
        };

        if self.daemon {
            runner
                .spawn(default_config, rest_addr, extra)
                .map_err(Error::Start)
        } else {
            runner
                .run(default_config, rest_addr, extra)
                .map_err(Error::Start)
        }
    }
}
