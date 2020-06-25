use crate::utils::{
    download, github,
    version::{Version, VersionReq},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Could not create the download client")]
    Client(#[source] download::Error),
    #[error("Could not get the releases data")]
    Release(#[source] github::Error),
}

pub fn check_jorup_update() -> Result<Option<github::Release>, Error> {
    let current_version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    let mut client = download::Client::new().map_err(Error::Client)?;
    let available_release =
        github::find_matching_release(&mut client, github::JORUP, VersionReq::Latest)
            .map_err(Error::Release)?;
    let res = if &current_version < available_release.version() {
        Some(available_release)
    } else {
        None
    };
    Ok(res)
}
