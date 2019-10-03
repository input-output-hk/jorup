use platforms::Platform;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub struct ReleaseBuilder {
    version: Option<Version>,
    assets: BTreeMap<String, Url>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Release {
    version: Version,
    assets: BTreeMap<String, Url>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Url(String);

#[derive(Debug, Clone)]
pub struct UrlBuilder {
    root: Option<String>,
    version: Option<Version>,
    platform: Option<Platform>,
}

impl UrlBuilder {
    pub fn root<S>(&mut self, root: S) -> &mut Self
    where
        S: Into<String>,
    {
        self.root = Some(root.into());
        self
    }
    pub fn version<V>(&mut self, version: V) -> &mut Self
    where
        V: Into<Version>,
    {
        self.version = Some(version.into());
        self
    }
    pub fn host_platform(&mut self) -> &mut Self {
        self.platform(platforms::guess_current().unwrap().clone())
    }
    pub fn platform(&mut self, platform: Platform) -> &mut Self {
        self.platform = Some(platform);
        self
    }
    pub fn build(&self) -> Url {
        let platform = self.platform.as_ref().unwrap();
        let package = if platform.target_os == platforms::target::OS::Windows {
            "zip"
        } else {
            "tar.gz"
        };
        Url(format!(
            "{root}/jormungandr/releases/download/v{version}/jormungandr-v{version}-{platform}.{package}",
            root = self.root.as_ref().unwrap(),
            version = self.version.as_ref().unwrap(),
            platform = platform.target_triple,
            package = package,
        ))
    }
}

impl ReleaseBuilder {
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.version = Some(version);
        self
    }

    pub fn add_assets<A, URL>(&mut self, asset: A, url: URL) -> &mut Self
    where
        A: Into<String>,
        URL: Into<Url>,
    {
        self.assets.insert(asset.into(), url.into());
        self
    }

    pub fn build(&self) -> Release {
        assert!(
            !self.assets.is_empty(),
            "missing assets for version {:?}",
            self.version
        );
        Release {
            version: self
                .version
                .clone()
                .expect("No version were given to the ReleaseBuilder"),
            assets: self.assets.clone(),
        }
    }
}

impl Release {
    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn assets(&self) -> &BTreeMap<String, Url> {
        &self.assets
    }
}

impl Default for ReleaseBuilder {
    fn default() -> Self {
        ReleaseBuilder {
            version: None,
            assets: BTreeMap::new(),
        }
    }
}

impl Default for UrlBuilder {
    fn default() -> Self {
        UrlBuilder {
            root: None,
            version: None,
            platform: None,
        }
    }
}

impl<T: Into<String>> From<T> for Url {
    fn from(url: T) -> Url {
        Url(url.into())
    }
}

impl AsRef<str> for Url {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// list of all currently known supported version of jormungandr
pub const AVAILABLE_PLATFORMS: &[Platform] = &[
    platforms::platform::tier1::X86_64_APPLE_DARWIN,
    platforms::platform::tier1::X86_64_PC_WINDOWS_GNU,
    platforms::platform::tier1::X86_64_PC_WINDOWS_MSVC,
    platforms::platform::tier1::X86_64_UNKNOWN_LINUX_GNU,
    platforms::platform::tier2::X86_64_UNKNOWN_LINUX_MUSL,
    platforms::platform::tier2::X86_64_UNKNOWN_NETBSD,
    platforms::platform::tier2::AARCH64_UNKNOWN_LINUX_GNU,
    platforms::platform::tier2::ARM_UNKNOWN_LINUX_GNUEABI,
    platforms::platform::tier2::ARMV7_UNKNOWN_LINUX_GNUEABIHF,
    platforms::platform::tier2::MIPS64EL_UNKNOWN_LINUX_GNUABI64,
    platforms::platform::tier2::POWERPC64LE_UNKNOWN_LINUX_GNU,
];
