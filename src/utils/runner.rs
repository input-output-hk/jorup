use crate::{utils::channel::Channel, utils::release::Release};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use thiserror::Error;

#[cfg(windows)]
use std::{error, fmt};

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

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot open file: {1}")]
    CannotOpenFile(#[source] io::Error, PathBuf),
    #[error("Cannot write file: {1}")]
    CannotWriteFile(#[source] io::Error, PathBuf),
    #[error("Cannot parse file: {1}")]
    Json(#[source] serde_json::Error, PathBuf),
    #[error("Cannot remove running file")]
    CannotRemoveRunnerFile(#[source] io::Error),
    #[error("Invalid version for jormungandr, Version ({0}) does not match requirement `{1}`")]
    InvalidJormungandrVersion(Version, VersionReq),
    #[error("Invalid version for jcli, Version ({0}) does not match requirement `{1}`")]
    InvalidJcliVersion(Version, VersionReq),
    #[error("Cannot start jormungandr")]
    CannotStartJormungandr(#[source] io::Error),
    #[error("No running node")]
    NoRunningNode,
    #[cfg(windows)]
    #[error("Cannot check id the node is running. Error code: {0}")]
    PidCheck(u64),
    #[cfg(unix)]
    #[error("Cannot check id the node is running")]
    PidCheck(#[source] io::Error),
    #[error("Node already running. PID: {0}")]
    NodeRunning(u32),
    #[error("Cannot stop running node")]
    CannotStopNode,
    #[error("Cannot send shutdown signal to the running node")]
    CannotSendStopSignal(#[source] io::Error),
    #[error("unable to create the address")]
    AddressCreate(#[source] io::Error),
    #[error("Invalid address")]
    InvalidAddress(#[source] std::string::FromUtf8Error),
    #[error("No secret key, did you mean to create a secret key too?")]
    NoSecretKey,
    #[error("Unable to extract the public key")]
    ReadPublicKey(#[from] io::Error),
    #[error("Cannot generate key {0}")]
    GenerateKey(String),
    #[error("Cannot get the version of {1}")]
    GetVersion(#[source] io::Error, PathBuf),
    #[error("Invalid output from execution of '{0} --version'")]
    InvalidVersionOutput(#[source] std::string::FromUtf8Error, PathBuf),
    #[error("Cannot parse the version of `{1}`: `{2}`")]
    ParseVersion(#[source] semver::SemVerError, PathBuf, String),
}

impl<'a, 'b> RunnerControl<'a, 'b> {
    pub fn new(channel: &'a Channel, release: &'b Release) -> Result<Self, Error> {
        let info_file = channel.get_runner_file();

        let info = if info_file.is_file() {
            let info = std::fs::read_to_string(&info_file)
                .map_err(|e| Error::CannotOpenFile(e, info_file.clone()))?;
            let info: RunnerInfo =
                serde_json::from_str(&info).map_err(|e| Error::Json(e, info_file))?;

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
                    .map_err(Error::CannotRemoveRunnerFile)?;
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

    pub fn override_jormungandr<P>(&mut self, jormungandr: P) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        let jormungandr = jormungandr.as_ref();
        let jormungandr = if jormungandr.is_relative() {
            std::env::current_dir().unwrap().join(jormungandr)
        } else {
            jormungandr.to_path_buf()
        };

        let version = get_version("jormungandr ", &jormungandr)?;
        let version_req = self.channel.entry().jormungandr_versions();

        if version_req.matches(&version) {
            self.jormungandr = Some(jormungandr);
            Ok(())
        } else {
            Err(Error::InvalidJormungandrVersion(
                version,
                version_req.clone(),
            ))
        }
    }

    pub fn jcli(&mut self) -> Result<Command, Error> {
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
            Err(Error::InvalidJcliVersion(version, version_req.clone()))
        }
    }

    pub fn jormungandr(&mut self) -> Result<Command, Error> {
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
            Err(Error::InvalidJormungandrVersion(
                version,
                version_req.clone(),
            ))
        }
    }

    fn prepare_config(&self) -> Result<(), Error> {
        let content = format!(
            r###"
# Generate file, do not update or change the values

rest:
    listen: "127.0.0.1:{}"
        "###,
            select_port_number()?
        );
        std::fs::write(self.channel.get_node_config(), content)
            .map_err(|e| Error::CannotWriteFile(e, self.channel.get_node_config()))
    }

    fn prepare(&mut self) -> Result<Command, Error> {
        self.prepare_config()?;
        let channel = self.channel;

        if let Some(info) = &self.info {
            return Err(Error::NodeRunning(info.pid));
        }

        let mut cmd = self.jormungandr()?;

        cmd.current_dir(channel.dir());

        let genesis_block_hash = std::fs::read_to_string(channel.get_genesis_block_hash()).unwrap();

        cmd.args(&[
            "--storage",
            channel.get_node_storage().display().to_string().as_str(),
            "--config",
            channel.get_node_config().display().to_string().as_str(),
            "--genesis-block-hash",
            &genesis_block_hash,
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

    pub fn spawn(&mut self, parameters: Vec<String>) -> Result<(), Error> {
        let mut cmd = self.prepare()?;
        cmd.args(parameters);

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(
            std::fs::File::create(self.channel.get_log_file())
                .map_err(|e| Error::CannotOpenFile(e, self.channel.get_log_file()))?,
        );

        let child = cmd.spawn().map_err(Error::CannotStartJormungandr)?;

        let runner_info = RunnerInfo {
            pid: child.id(),
            rest_port: 8080,
        };

        std::fs::write(
            self.channel.get_runner_file(),
            serde_json::to_string(&runner_info).unwrap(),
        )
        // TODO? on failure, shall we kill the child?
        .map_err(|e| Error::CannotWriteFile(e, self.channel.get_runner_file()))?;

        self.info = Some(runner_info);

        Ok(())
    }

    pub fn run(mut self, parameters: Vec<String>) -> Result<(), Error> {
        let mut cmd = self.prepare()?;
        cmd.args(parameters);
        let mut child = cmd.spawn().map_err(Error::CannotStartJormungandr)?;

        child
            .wait()
            .map(|status| println!("exit status: {}", status))
            .map_err(|e| panic!("failed to wait for exit: {}", e))
    }

    pub fn shutdown(&mut self) -> Result<(), Error> {
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
            .map_err(Error::CannotSendStopSignal)?;

        if status.success() {
            std::fs::remove_file(self.channel.get_runner_file())
                .map_err(Error::CannotRemoveRunnerFile)
        } else {
            Err(Error::CannotStopNode)
        }
    }

    pub fn settings(&mut self) -> Result<(), Error> {
        let info = if let Some(info) = &self.info {
            info.clone()
        } else {
            return Err(Error::NoRunningNode);
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
            .map_err(Error::CannotSendStopSignal)?;

        if status.success() {
            Ok(())
        } else {
            Err(Error::CannotStopNode)
        }
    }

    pub fn info(&mut self) -> Result<(), Error> {
        let info = if let Some(info) = &self.info {
            info.clone()
        } else {
            return Err(Error::NoRunningNode);
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
            .map_err(Error::CannotSendStopSignal)?;

        if status.success() {
            Ok(())
        } else {
            Err(Error::CannotStopNode)
        }
    }

    pub fn get_wallet_secret_key(&mut self, force: bool) -> Result<PathBuf, Error> {
        let wallet_path = self.channel.get_wallet_secret();

        if !wallet_path.is_file() || force {
            self.gen_secret_key("Ed25519", &wallet_path)?;
        }

        Ok(wallet_path)
    }

    pub fn get_wallet_address(&mut self) -> Result<String, Error> {
        let pk = self.get_public_key(self.channel.get_wallet_secret())?;

        let address = self.make_address(pk.trim_end())?;

        Ok(address.trim_end().to_owned())
    }

    fn make_address<PK: AsRef<str>>(&mut self, public_key: PK) -> Result<String, Error> {
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
            .map_err(Error::AddressCreate)?;
        String::from_utf8(output.stdout).map_err(Error::InvalidAddress)
    }

    fn get_public_key<P>(&mut self, secret_key: P) -> Result<String, Error>
    where
        P: AsRef<Path>,
    {
        if !secret_key.as_ref().is_file() {
            return Err(Error::NoSecretKey);
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
            .map_err(Error::ReadPublicKey)?;

        String::from_utf8(output.stdout).map_err(Error::InvalidAddress)
    }

    fn gen_secret_key<P>(&mut self, key_type: &str, path: P) -> Result<(), Error>
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
            .map_err(|_| Error::GenerateKey(key_type.to_owned()))?;
        if status.success() {
            Ok(())
        } else {
            return Err(Error::GenerateKey(key_type.to_owned()));
        }
    }
}

#[cfg(unix)]
fn check_pid(pid: u32) -> Result<bool, Error> {
    let status = Command::new("ps")
        .arg(&pid.to_string())
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(Error::PidCheck)?;

    Ok(status.success())
}

#[cfg(windows)]
fn check_pid(pid: u32) -> Result<bool, Error> {
    use winapi::{
        shared::minwindef::*,
        um::{
            errhandlingapi::GetLastError,
            minwinbase::*,
            processthreadsapi::{GetExitCodeProcess, OpenProcess},
            winnt::*,
        },
    };

    unsafe {
        let process_handle = OpenProcess(PROCESS_QUERY_INFORMATION, TRUE, pid as DWORD);

        let mut exit_code: DWORD = 0;
        let check_status = GetExitCodeProcess(process_handle, &mut exit_code as *mut DWORD);

        if check_status == TRUE {
            Ok(exit_code == STILL_ACTIVE)
        } else {
            let error_code = GetLastError();
            Err(Error::PidCheck(error_code as u64))
        }
    }
}

fn get_version<P>(executable: &str, cmd: P) -> Result<Version, Error>
where
    P: AsRef<Path>,
{
    let output = Command::new(cmd.as_ref())
        .arg("--version")
        .output()
        .map_err(|e| Error::GetVersion(e, cmd.as_ref().to_path_buf()))?;

    let output = String::from_utf8(output.stdout)
        .map_err(|e| Error::InvalidVersionOutput(e, cmd.as_ref().to_path_buf()))?;

    output
        .trim_start_matches(executable)
        .parse()
        .map_err(|e| Error::ParseVersion(e, cmd.as_ref().to_path_buf(), output))
}

fn select_port_number() -> Result<u16, Error> {
    // TODO
    Ok(8080)
}
