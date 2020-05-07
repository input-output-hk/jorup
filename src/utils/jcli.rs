use crate::utils::blockchain::Blockchain;
use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
};
use thiserror::Error;

/// jcli interactions that do not require a running jormungandr node
#[derive(Debug, Error)]
pub enum Error {
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
}

pub struct Jcli<'a> {
    blockchain: &'a Blockchain,
    path: PathBuf,
}

impl<'a> Jcli<'a> {
    pub fn new(blockchain: &'a Blockchain, path: PathBuf) -> Self {
        Self { blockchain, path }
    }

    fn command(&self) -> Command {
        Command::new(&self.path)
    }

    pub fn get_wallet_secret_key(&mut self, force: bool) -> Result<PathBuf, Error> {
        let wallet_path = self.blockchain.get_wallet_secret();

        if !wallet_path.is_file() || force {
            self.gen_secret_key("Ed25519", &wallet_path)?;
        }

        Ok(wallet_path)
    }

    pub fn get_wallet_address(&mut self) -> Result<String, Error> {
        let pk = self.get_public_key(self.blockchain.get_wallet_secret())?;

        let address = self.make_address(pk.trim_end())?;

        Ok(address.trim_end().to_owned())
    }

    fn make_address<PK: AsRef<str>>(&mut self, public_key: PK) -> Result<String, Error> {
        let output = self
            .command()
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
            .command()
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
            .command()
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
