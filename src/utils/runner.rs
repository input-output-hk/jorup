use crate::utils::blockchain::Blockchain;
use serde::{Deserialize, Serialize};
use std::{
    io,
    net::SocketAddr,
    path::PathBuf,
    process::{Child, Command, Stdio},
};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunnerInfo {
    pid: u32,
    rest_port: Option<u16>,
    jcli: PathBuf,
    jormungandr: PathBuf,
}

pub struct RunnerControl<'a> {
    blockchain: &'a Blockchain,
    info: Option<RunnerInfo>,
    jcli: PathBuf,
    jormungandr: PathBuf,
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
    #[error("Request to a running node failed")]
    CannotPerrformRequest,
    #[error("Cannot send shutdown signal to the running node")]
    CannotSendStopSignal(#[source] io::Error),
    #[error("REST is not running")]
    RestNotRunning,
}

impl<'a> RunnerControl<'a> {
    pub fn new(blockchain: &'a Blockchain, bin_dir: PathBuf) -> Result<Self, Error> {
        let info_file = blockchain.get_runner_file();

        if info_file.is_file() {
            let info = std::fs::read_to_string(&info_file)
                .map_err(|e| Error::CannotOpenFile(e, info_file.clone()))?;
            let info: RunnerInfo =
                serde_json::from_str(&info).map_err(|e| Error::Json(e, info_file))?;

            let is_up = check_pid(info.pid)?;

            if is_up {
                return Err(Error::NodeRunning(info.pid));
            }

            eprintln!("WARN: removing previous runner file");
            eprintln!("      it seems a previous node was not shutdown properly");
            eprintln!(
                "      check {} for more information",
                blockchain.get_log_file().display()
            );
            std::fs::remove_file(blockchain.get_runner_file())
                .map_err(Error::CannotRemoveRunnerFile)?;
        }

        Ok(RunnerControl {
            blockchain,
            info: None,
            jcli: bin_dir.join("jcli"),
            jormungandr: bin_dir.join("jormungandr"),
        })
    }

    pub fn load(blockchain: &'a Blockchain) -> Result<Self, Error> {
        let info_file = blockchain.get_runner_file();

        if !info_file.is_file() {
            return Err(Error::NoRunningNode);
        }

        let info = std::fs::read_to_string(&info_file)
            .map_err(|e| Error::CannotOpenFile(e, info_file.clone()))?;
        let info: RunnerInfo =
            serde_json::from_str(&info).map_err(|e| Error::Json(e, info_file))?;

        let is_up = check_pid(info.pid)?;

        if !is_up {
            return Err(Error::NoRunningNode);
        }

        let jcli = info.jcli.clone();
        let jormungandr = info.jormungandr.clone();

        return Ok(RunnerControl {
            blockchain,
            info: Some(info),
            jcli,
            jormungandr,
        });
    }

    pub fn jcli(&self) -> Command {
        Command::new(&self.jcli)
    }

    pub fn jormungandr(&self) -> Command {
        Command::new(&self.jormungandr)
    }

    fn prepare(
        &mut self,
        default_config: bool,
        rest_addr: Option<SocketAddr>,
        parameters: Vec<String>,
        cin: Stdio,
        cout: Stdio,
        cerr: Stdio,
    ) -> Result<Child, Error> {
        let blockchain = self.blockchain;

        if let Some(info) = &self.info {
            return Err(Error::NodeRunning(info.pid));
        }

        let mut cmd = self.jormungandr();

        cmd.current_dir(blockchain.dir());

        if let Some(rest_addr) = rest_addr {
            cmd.args(&["--rest-listen", &rest_addr.to_string()]);
        }

        if default_config {
            let genesis_block_hash =
                std::fs::read_to_string(blockchain.get_genesis_block_hash()).unwrap();

            cmd.args(&[
                "--storage",
                blockchain.get_node_storage().display().to_string().as_str(),
                "--genesis-block-hash",
                &genesis_block_hash,
            ]);

            for peer in blockchain.entry().trusted_peers() {
                cmd.args(&[
                    "--trusted-peer",
                    &format!("{}@{}", peer.address(), peer.id()),
                ]);
            }

            if blockchain.get_node_secret().is_file() {
                cmd.args(&[
                    "--secret",
                    blockchain.get_node_secret().display().to_string().as_str(),
                ]);
            }
        }

        cmd.args(parameters);

        cmd.stdin(cin);
        cmd.stdout(cout);
        cmd.stderr(cerr);

        let child = cmd.spawn().map_err(Error::CannotStartJormungandr)?;

        let runner_info = RunnerInfo {
            pid: child.id(),
            rest_port: rest_addr.as_ref().map(|rest| rest.port()),
            jcli: self.jcli.clone(),
            jormungandr: self.jormungandr.clone(),
        };

        std::fs::write(
            self.blockchain.get_runner_file(),
            serde_json::to_string(&runner_info).unwrap(),
        )
        // TODO? on failure, shall we kill the child?
        .map_err(|e| Error::CannotWriteFile(e, self.blockchain.get_runner_file()))?;

        self.info = Some(runner_info);

        Ok(child)
    }

    pub fn spawn(
        &mut self,
        default_config: bool,
        rest_addr: Option<SocketAddr>,
        parameters: Vec<String>,
    ) -> Result<(), Error> {
        let cerr = std::fs::File::create(self.blockchain.get_log_file())
            .map_err(|e| Error::CannotOpenFile(e, self.blockchain.get_log_file()))?;

        let _child = self.prepare(
            default_config,
            rest_addr,
            parameters,
            Stdio::null(),
            Stdio::null(),
            Stdio::from(cerr),
        )?;

        Ok(())
    }

    pub fn run(
        mut self,
        default_config: bool,
        rest_addr: Option<SocketAddr>,
        parameters: Vec<String>,
    ) -> Result<(), Error> {
        let mut child = self.prepare(
            default_config,
            rest_addr,
            parameters,
            Stdio::inherit(),
            Stdio::inherit(),
            Stdio::inherit(),
        )?;

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
            .jcli()
            .args(&[
                "rest",
                "v0",
                "shutdown",
                "get",
                "--host",
                &format!(
                    "http://localhost:{}/api",
                    info.rest_port.ok_or(Error::RestNotRunning)?
                ),
            ])
            .status()
            .map_err(Error::CannotSendStopSignal)?;

        if status.success() {
            std::fs::remove_file(self.blockchain.get_runner_file())
                .map_err(Error::CannotRemoveRunnerFile)
        } else {
            Err(Error::CannotPerrformRequest)
        }
    }

    pub fn settings(&mut self) -> Result<(), Error> {
        let info = if let Some(info) = &self.info {
            info.clone()
        } else {
            return Err(Error::NoRunningNode);
        };

        let status = self
            .jcli()
            .args(&[
                "rest",
                "v0",
                "settings",
                "get",
                "--host",
                &format!(
                    "http://localhost:{}/api",
                    info.rest_port.ok_or(Error::RestNotRunning)?
                ),
            ])
            .status()
            .map_err(Error::CannotSendStopSignal)?;

        if status.success() {
            Ok(())
        } else {
            Err(Error::CannotPerrformRequest)
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
            .jcli()
            .args(&[
                "rest",
                "v0",
                "node",
                "stats",
                "get",
                "--host",
                &format!(
                    "http://localhost:{}/api",
                    info.rest_port.ok_or(Error::RestNotRunning)?
                ),
            ])
            .status()
            .map_err(Error::CannotSendStopSignal)?;

        if status.success() {
            Ok(())
        } else {
            Err(Error::CannotPerrformRequest)
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
