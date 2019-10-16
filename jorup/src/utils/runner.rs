use crate::{utils::channel::Channel, utils::release::Release};
use jorup_lib::Version;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
};
use tokio::prelude::*;
use tokio_process::CommandExt as _;

error_chain! {}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunnerInfo {
    pid: u32,
    rest_port: u16,
}

pub struct RunnerControl<'a, 'b> {
    channel: &'a Channel,
    release: &'b Release,
    info: Option<RunnerInfo>,
    jcli: Option<PathBuf>,
    jormungandr: Option<PathBuf>,
}

impl<'a, 'b> RunnerControl<'a, 'b> {
    pub fn new(channel: &'a Channel, release: &'b Release) -> Result<Self> {
        let info_file = channel.get_runner_file();

        let info = if info_file.is_file() {
            let info = std::fs::read_to_string(&info_file)
                .chain_err(|| format!("Cannot open file {}", info_file.display()))?;
            let info: RunnerInfo = toml::from_str(&info)
                .chain_err(|| format!("Cannot parse file {}", info_file.display()))?;

            let is_up = check_pid(info.pid)?;

            if is_up {
                Some(info)
            } else {
                eprintln!("WARN: removing previous runner file");
                eprintln!("      it seems a previous node was not shutdown properly");
                eprintln!(
                    "      check {} for more information",
                    channel.get_log_file().display()
                );
                std::fs::remove_file(channel.get_runner_file())
                    .chain_err(|| "Cannot remove the running file")?;
                None
            }
        } else {
            None
        };

        Ok(RunnerControl {
            channel,
            release,
            info,
            jcli: None,
            jormungandr: None,
        })
    }

    pub fn jcli(&mut self) -> Result<Command> {
        if let Some(jcli) = &self.jcli {
            return Ok(Command::new(jcli));
        }

        let jcli = self.release.get_jcli();

        let version = get_version("jcli ", &jcli)?;
        let version_req = self.channel.entry().jormungandr_versions();

        if version_req.matches(&version) {
            let cmd = Command::new(&jcli);
            self.jcli = Some(jcli);
            Ok(cmd)
        } else {
            bail!(
                "Invalid version for jcli, Version ({}) does not match requirement `{}`",
                version,
                version_req
            )
        }
    }

    pub fn jormungandr(&mut self) -> Result<Command> {
        if let Some(jormungandr) = &self.jormungandr {
            return Ok(Command::new(jormungandr));
        }

        let jormungandr = self.release.get_jormungandr();

        let version = get_version("jormungandr ", &jormungandr)?;
        let version_req = self.channel.entry().jormungandr_versions();

        if version_req.matches(&version) {
            let cmd = Command::new(&jormungandr);
            self.jormungandr = Some(jormungandr);
            Ok(cmd)
        } else {
            bail!(
                "Invalid version for jormungandr, Version ({}) does not match requirement `{}`",
                version,
                version_req
            )
        }
    }

    fn prepare_config(&self) -> Result<()> {
        let content = format!(
            r###"
# Generate file, do not update or change the values

rest:
    listen: "127.0.0.1:{}"
        "###,
            select_port_number()?
        );
        std::fs::write(self.channel.get_node_config(), content).chain_err(|| {
            format!(
                "Cannot write node's config ({})",
                self.channel.get_node_config().display()
            )
        })
    }

    fn prepare(&mut self) -> Result<Command> {
        self.prepare_config()?;
        let channel = self.channel;

        if let Some(info) = &self.info {
            bail!(format!("Not already running ({})", info.pid))
        }

        let mut cmd = self.jormungandr()?;

        cmd.current_dir(channel.dir());

        cmd.args(&[
            "--storage",
            channel.get_node_storage().display().to_string().as_str(),
            "--config",
            channel.get_node_config().display().to_string().as_str(),
            "--genesis-block",
            channel.get_genesis_block().display().to_string().as_str(),
        ]);

        for peer in channel.entry().known_trusted_peers() {
            cmd.args(&[
                "--trusted-peer",
                &format!("{}@{}", peer.address(), peer.id()),
            ]);
        }

        if channel.get_node_secret().is_file() {
            cmd.args(&[
                "--secret",
                channel.get_node_secret().display().to_string().as_str(),
            ]);
        }

        Ok(cmd)
    }

    pub fn spawn(&mut self) -> Result<()> {
        let mut cmd = self.prepare()?;

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(
            std::fs::File::create(self.channel.get_log_file())
                .chain_err(|| "Cannot create/open log file")?,
        );

        let child = cmd.spawn().chain_err(|| "Cannot start jormungandr")?;

        let runner_info = RunnerInfo {
            pid: child.id(),
            rest_port: 8080,
        };

        std::fs::write(
            self.channel.get_runner_file(),
            toml::to_string(&runner_info).unwrap(),
        )
        // TODO? on failure, shall we kill the child?
        .chain_err(|| "Cannot save the process info file")?;

        self.info = Some(runner_info);

        Ok(())
    }

    pub fn run(mut self) -> Result<()> {
        let mut cmd = self.prepare()?;
        let child = cmd.spawn_async().chain_err(|| "Cannot start jormungandr")?;

        tokio::run(
            child
                .map(|status| println!("exit status: {}", status))
                .map_err(|e| panic!("failed to wait for exit: {}", e)),
        );
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<()> {
        let info = if let Some(info) = std::mem::replace(&mut self.info, None) {
            info
        } else {
            return Ok(());
        };

        let status = self
            .jcli()?
            .args(&[
                "rest",
                "v0",
                "shutdown",
                "get",
                "--host",
                &format!("http://localhost:{}/api", info.rest_port),
            ])
            .status()
            .chain_err(|| "Cannot send shutdown signal to the running node")?;

        if status.success() {
            std::fs::remove_file(self.channel.get_runner_file())
                .chain_err(|| "Cannot remove the running file")
        } else {
            bail!("Cannot stop running node? Invalid state")
        }
    }

    pub fn settings(&mut self) -> Result<()> {
        let info = if let Some(info) = &self.info {
            info.clone()
        } else {
            bail!("No running node")
        };

        let status = self
            .jcli()?
            .args(&[
                "rest",
                "v0",
                "settings",
                "get",
                "--host",
                &format!("http://localhost:{}/api", info.rest_port),
            ])
            .status()
            .chain_err(|| "Cannot send shutdown signal to the running node")?;

        if status.success() {
            Ok(())
        } else {
            bail!("Cannot stop running node? Invalid state")
        }
    }

    pub fn info(&mut self) -> Result<()> {
        let info = if let Some(info) = &self.info {
            info.clone()
        } else {
            bail!("No running node")
        };

        println!("{:#?}", info);

        let status = self
            .jcli()?
            .args(&[
                "rest",
                "v0",
                "node",
                "stats",
                "get",
                "--host",
                &format!("http://localhost:{}/api", info.rest_port),
            ])
            .status()
            .chain_err(|| "Cannot send shutdown signal to the running node")?;

        if status.success() {
            Ok(())
        } else {
            bail!("Cannot stop running node? Invalid state")
        }
    }

    pub fn get_wallet_secret_key(&mut self, force: bool) -> Result<PathBuf> {
        let wallet_path = self.channel.get_wallet_secret();

        if !wallet_path.is_file() || force {
            self.gen_secret_key("Ed25519", &wallet_path)?;
        }

        Ok(wallet_path)
    }

    pub fn get_wallet_address(&mut self) -> Result<String> {
        let pk = self.get_public_key(self.channel.get_wallet_secret())?;

        let address = self.make_address(pk.trim_end())?;

        Ok(address.trim_end().to_owned())
    }

    fn make_address<PK: AsRef<str>>(&mut self, public_key: PK) -> Result<String> {
        let output = self
            .jcli()?
            .args(&[
                "address",
                "account",
                "--testing",
                "--prefix=jorup_",
                public_key.as_ref(),
            ])
            .output()
            .chain_err(|| "unable to create the address")?;
        String::from_utf8(output.stdout).chain_err(|| "Invalid address")
    }

    fn get_public_key<P>(&mut self, secret_key: P) -> Result<String>
    where
        P: AsRef<Path>,
    {
        if !secret_key.as_ref().is_file() {
            bail!("No secret key, did you mean to create a secret key too?")
        }

        let output = self
            .jcli()?
            .args(&[
                "key",
                "to-public",
                "--input",
                secret_key.as_ref().display().to_string().as_str(),
            ])
            .output()
            .chain_err(|| "unable to extract the public key")?;

        String::from_utf8(output.stdout).chain_err(|| "Invalid public key")
    }

    fn gen_secret_key<P>(&mut self, key_type: &str, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let status = self
            .jcli()?
            .args(&[
                "key",
                "generate",
                "--type",
                key_type,
                path.as_ref().display().to_string().as_str(),
            ])
            .status()
            .chain_err(|| format!("Cannot generate key {}", key_type))?;
        if status.success() {
            Ok(())
        } else {
            bail!(format!("Cannot generate key {}", key_type))
        }
    }
}

#[cfg(unix)]
fn check_pid(pid: u32) -> Result<bool> {
    let status = Command::new("ps")
        .arg(&pid.to_string())
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .chain_err(|| "Cannot check process ID is running")?;

    Ok(status.success())
}

fn get_version<P>(executable: &str, cmd: P) -> Result<Version>
where
    P: AsRef<Path>,
{
    let output = Command::new(cmd.as_ref())
        .arg("--version")
        .output()
        .chain_err(|| format!("Cannot get the version of {}", cmd.as_ref().display()))?;

    let output = String::from_utf8(output.stdout).chain_err(|| {
        format!(
            "Invalid output from execution of '{} --version'",
            cmd.as_ref().display()
        )
    })?;

    output.trim_start_matches(executable).parse().chain_err(|| {
        format!(
            "Cannot parse the version of `{}`: `{}`",
            cmd.as_ref().display(),
            output
        )
    })
}

fn select_port_number() -> Result<u16> {
    // TODO
    Ok(8080)
}
