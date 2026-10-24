use crate::{BuildOptions, DevelopmentOrigin};
use std::ffi::OsString;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevServerOptions {
    pub host: IpAddr,
    pub port: u16,
    pub allow_non_loopback: bool,
    pub output_directory: PathBuf,
    pub watch: bool,
    pub build: bool,
    pub restart_executable: Option<PathBuf>,
    pub restart_check_arguments: Vec<OsString>,
    pub native: Option<DevelopmentOrigin>,
}

impl Default for DevServerOptions {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 8000,
            allow_non_loopback: false,
            output_directory: PathBuf::from("dist"),
            watch: true,
            build: true,
            restart_executable: None,
            restart_check_arguments: vec![OsString::from("--version")],
            native: None,
        }
    }
}

impl DevServerOptions {
    pub(in crate::server) fn build_options(&self) -> BuildOptions {
        BuildOptions {
            output_directory: self.output_directory.clone(),
            profile: crate::BuildProfile::Development,
            ..BuildOptions::default()
        }
    }

    pub(in crate::server) fn site_builder(&self) -> super::SiteBuilder {
        super::SiteBuilder::new(self.build_options(), self.native.clone())
    }
}
