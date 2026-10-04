use crate::Failure;
use crate::failure::diagnostic;
use crate::safety::{ServerHost, TrustedProxies};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt, OpenOptionsSyncExt};
use cap_std::fs::{Dir, File, Metadata, OpenOptions};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt;
use std::io::{ErrorKind, Read};
use std::net::IpAddr;
use std::path::Path;

mod parser;
mod secrets;
mod shape;
#[cfg(test)]
mod tests;

type Environment<'a> = dyn Fn(&str) -> Result<Option<String>, Failure> + 'a;

#[derive(Clone, Default, Eq, PartialEq)]
pub struct ApplicationConfig {
    host: Option<ServerHost>,
    port: Option<u16>,
    hosts: Vec<String>,
    trusted_proxies: TrustedProxies,
    handler_json: Option<String>,
    topics_directory: Option<String>,
}

impl fmt::Debug for ApplicationConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApplicationConfig")
            .field("configured", &self.handler_json.is_some())
            .finish_non_exhaustive()
    }
}

impl ApplicationConfig {
    const MAX_BYTES: usize = 65_536;
    const MANIFEST: &str = "client/manifest.json";
    const APPLICATION: &str = "server/application.json";
    const MAX_RESOLVED_BYTES: usize = 16_384;
    const MAX_KEYS: usize = 128;
    const HOSTS_MESSAGE: &str = "yon.server.hosts must be a list of host names without scheme, port or path, such as \"app.example.com\" or \"*.example.com\".";
    const TRUSTED_PROXIES_MESSAGE: &str = "yon.server.proxies must be a list of proxy IP addresses or CIDR ranges, such as \"10.0.0.5\" or \"10.0.0.0/8\" (ADR 0096).";
    const TAC_CONFIG_MOVED: &str = "tac.config is no longer read; client/manifest.json is the Tac configuration (ADR 0052). Move application to tac.application (name and entryRoute become name and start_url), scripts, styles and cache to tac.scripts, tac.styles and tac.cache, metadata to tac.metadata.routes, and origin to tac.origin. postBundle is replaced by the built-in sitemap.xml and robots.txt; list dynamic route values in tac.sitemap. `ty migrate fix --write` moves plain values.";

    pub fn load(root: &Path) -> Result<Self, Failure> {
        Self::load_for_server(root, None, None)
    }

    pub fn load_for_server(
        root: &Path,
        host: Option<ServerHost>,
        port: Option<u16>,
    ) -> Result<Self, Failure> {
        Self::load_with_endpoint_overrides(root, &read_environment, host, port)
    }

    pub fn load_project_for_server(
        project: &crate::Project,
        host: Option<ServerHost>,
        port: Option<u16>,
    ) -> Result<Self, Failure> {
        Self::load_directory_for_server(&project.capability(), host, port)
    }

    pub(crate) fn load_directory_for_server(
        directory: &Dir,
        host: Option<ServerHost>,
        port: Option<u16>,
    ) -> Result<Self, Failure> {
        Self::load_opened(directory, &read_environment, host, port)
    }

    fn load_with_endpoint_overrides(
        root: &Path,
        environment: &Environment<'_>,
        host: Option<ServerHost>,
        port: Option<u16>,
    ) -> Result<Self, Failure> {
        let directory = open_directory(root)?;
        Self::load_opened(&directory, environment, host, port)
    }

    pub fn load_with_environment(
        root: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<Self, Failure> {
        Self::load_with_endpoint_overrides(
            root,
            &|name| Ok(environment.get(name).cloned()),
            None,
            None,
        )
    }

    #[must_use]
    pub fn host(&self) -> Option<IpAddr> {
        self.host.as_ref().map(ServerHost::address)
    }

    #[must_use]
    pub const fn server_host(&self) -> Option<&ServerHost> {
        self.host.as_ref()
    }

    #[must_use]
    pub const fn port(&self) -> Option<u16> {
        self.port
    }

    #[must_use]
    pub fn hosts(&self) -> &[String] {
        &self.hosts
    }

    pub(crate) const fn trusted_proxies(&self) -> &TrustedProxies {
        &self.trusted_proxies
    }

    #[must_use]
    pub fn handler_json(&self) -> Option<&str> {
        self.handler_json.as_deref()
    }

    #[must_use]
    pub fn topics_directory(&self) -> Option<&str> {
        self.topics_directory.as_deref()
    }

    pub fn load_client(root: &Path) -> Result<Self, Failure> {
        Self::load_directory_for_client(&open_directory(root)?)
    }

    pub fn load_project_for_client(project: &crate::Project) -> Result<Self, Failure> {
        Self::load_directory_for_client(&project.capability())
    }

    pub(crate) fn load_directory_for_client(directory: &Dir) -> Result<Self, Failure> {
        reject_legacy_locations(directory)?;
        read_document(directory, Self::APPLICATION, false)?;
        let mut public = Map::new();
        if let Some(bytes) = read_bytes(directory, Self::MANIFEST, false)? {
            let manifest = parser::parse(&bytes)?;
            if let Some(tac) = manifest.get("tac") {
                public.insert("tac".into(), without_framework_members(tac));
            }
        }
        let public = Value::Object(public);
        shape::validate(&public)?;
        Self::from_document(public, None, None)
    }

    fn load_opened(
        directory: &Dir,
        environment: &Environment<'_>,
        host: Option<ServerHost>,
        port: Option<u16>,
    ) -> Result<Self, Failure> {
        reject_legacy_locations(directory)?;
        let mut document = read_document(directory, Self::APPLICATION, false)?
            .unwrap_or_else(|| Value::Object(Map::new()));
        if let Some(profile) = environment("YON_PROFILE")? {
            let selected = read_profile(directory, &profile)?;
            merge(&mut document, selected);
        }
        secrets::merge_file(directory, &mut document)?;
        Self::from_document(document, host, port)
    }

    fn from_document(
        mut document: Value,
        explicit_host: Option<ServerHost>,
        explicit_port: Option<u16>,
    ) -> Result<Self, Failure> {
        parser::validate_bounds(&document)?;
        let host = match explicit_host {
            Some(host) => Some(host),
            None => parse_host(document.pointer("/yon/server/host"))?,
        };
        let port = match explicit_port {
            Some(port) => Some(port),
            None => parse_port(document.pointer("/yon/server/port"))?,
        };
        let hosts = Self::hosts_from(document.pointer("/yon/server/hosts"))?;
        let trusted_proxies =
            Self::trusted_proxies_from(document.pointer("/yon/server/proxies"))?;
        let topics_directory =
            Self::topics_directory_from(document.pointer("/yon/topics/directory"))?;
        normalize_endpoint(&mut document, host.as_ref(), port);
        parser::validate_bounds(&document)?;
        let serialized = serde_json::to_string(&document)
            .map_err(|_| invalid("Cannot encode application configuration."))?;
        if serialized.len() > Self::MAX_RESOLVED_BYTES {
            return Err(invalid(
                "Resolved application configuration exceeds the 16 KiB handler transport limit.",
            ));
        }
        let handler_json = (serialized != "{}").then_some(serialized);
        Ok(Self {
            host,
            port,
            hosts,
            trusted_proxies,
            handler_json,
            topics_directory,
        })
    }

    fn hosts_from(value: Option<&Value>) -> Result<Vec<String>, Failure> {
        let entries = value
            .map_or(Some(&[][..]), |value| value.as_array().map(Vec::as_slice))
            .ok_or_else(|| invalid(Self::HOSTS_MESSAGE))?;
        entries.iter().map(Self::host_entry).collect()
    }

    fn host_entry(entry: &Value) -> Result<String, Failure> {
        entry
            .as_str()
            .filter(|name| crate::safety::DeclaredHosts::is_name_entry(name))
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| invalid(Self::HOSTS_MESSAGE))
    }

    fn trusted_proxies_from(value: Option<&Value>) -> Result<TrustedProxies, Failure> {
        let entries = value
            .map_or(Some(&[][..]), |value| value.as_array().map(Vec::as_slice))
            .ok_or_else(|| invalid(Self::TRUSTED_PROXIES_MESSAGE))?;
        let texts = entries
            .iter()
            .map(Value::as_str)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| invalid(Self::TRUSTED_PROXIES_MESSAGE))?;
        TrustedProxies::parse(&texts).ok_or_else(|| invalid(Self::TRUSTED_PROXIES_MESSAGE))
    }

    fn topics_directory_from(value: Option<&Value>) -> Result<Option<String>, Failure> {
        let Some(value) = value else {
            return Ok(None);
        };
        let directory = value
            .as_str()
            .ok_or_else(|| invalid("topics.directory must be a folder path written as text."))?;
        Ok(Some(String::from(directory)))
    }
}

fn without_framework_members(tac: &Value) -> Value {
    let Some(members) = tac.as_object() else {
        return tac.clone();
    };
    let mut configuration = members.clone();
    for name in crate::native::TacMember::FRAMEWORK_MEMBERS {
        configuration.remove(name);
    }
    Value::Object(configuration)
}

fn reject_legacy_locations(directory: &Dir) -> Result<(), Failure> {
    for (name, message) in [
        (
            "application.json",
            "application.json moved to server/application.json, beside the server it configures (ADR 0034).",
        ),
        (
            "manifest.json",
            "manifest.json moved to client/manifest.json, beside the client it describes (ADR 0034).",
        ),
        (
            "tachyon.json",
            "tachyon.json is no longer read. Move the application id and version into client/manifest.json under tac.application; name and entry route come from name and start_url (ADR 0052). `ty migrate fix --write` moves them.",
        ),
        ("tac.config.js", ApplicationConfig::TAC_CONFIG_MOVED),
        ("tac.config.mjs", ApplicationConfig::TAC_CONFIG_MOVED),
        ("tac.config.ts", ApplicationConfig::TAC_CONFIG_MOVED),
    ] {
        if directory.symlink_metadata(name).is_ok() {
            return Err(invalid(message));
        }
    }
    let entries = directory
        .entries()
        .map_err(|_| invalid("Cannot list the project directory."))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("application-") && name.ends_with(".json") {
            return Err(invalid(
                "Profile files moved beside application.json, to server/application-<profile>.json (ADR 0034).",
            ));
        }
    }
    Ok(())
}

fn open_directory(root: &Path) -> Result<Dir, Failure> {
    Dir::open_ambient_dir(root, cap_std::ambient_authority())
        .map_err(|_| invalid("Cannot open the application configuration directory."))
}

#[cfg(feature = "fuzzing")]
pub(crate) fn fuzz(input: &[u8]) {
    if input.len() > ApplicationConfig::MAX_BYTES {
        return;
    }
    let Ok(mut document) = parser::parse(input) else {
        return;
    };
    let profile = document.clone();
    merge(&mut document, profile);
    if shape::validate(&document).is_ok() {
        let _ = ApplicationConfig::from_document(document, None, None);
    }
}

fn read_document(directory: &Dir, name: &str, required: bool) -> Result<Option<Value>, Failure> {
    let Some(bytes) = read_bytes(directory, name, required)? else {
        return Ok(None);
    };
    let document = parser::parse(&bytes)?;
    if document.get("tac").is_some() {
        return Err(invalid(
            "server/application.json and its profiles configure Yon only; move the tac section to the tac member of client/manifest.json, the web app manifest (ADR 0034).",
        ));
    }
    if document.get("yon").is_some() {
        return Err(invalid(
            "server/application.json is Yon's namespace itself; remove the yon root key and keep its members at the top level (ADR 0034).",
        ));
    }
    let document = wrap_in_yon_namespace(document);
    shape::validate(&document)?;
    Ok(Some(document))
}

fn wrap_in_yon_namespace(document: Value) -> Value {
    if document.as_object().is_some_and(Map::is_empty) {
        return document;
    }
    let mut namespaced = Map::new();
    namespaced.insert("yon".into(), document);
    Value::Object(namespaced)
}

fn read_profile(directory: &Dir, profile: &str) -> Result<Value, Failure> {
    validate_profile(profile)?;
    let name = format!("server/application-{profile}.json");
    read_document(directory, &name, true)?
        .ok_or_else(|| invalid("The selected application profile file is missing."))
}

fn read_bytes(directory: &Dir, name: &str, required: bool) -> Result<Option<Vec<u8>>, Failure> {
    let Some(metadata) = file_metadata(directory, name, required)? else {
        return Ok(None);
    };
    if !metadata.is_file() || metadata.is_symlink() {
        return Err(invalid(
            "Application configuration must be a regular, non-symlink file.",
        ));
    }
    if metadata.len() > ApplicationConfig::MAX_BYTES as u64 {
        return Err(invalid("Application configuration exceeds 64 KiB."));
    }
    let file = open_regular_file(directory, name)?;
    read_limited(file).map(Some)
}

fn file_metadata(directory: &Dir, name: &str, required: bool) -> Result<Option<Metadata>, Failure> {
    match directory.symlink_metadata(name) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if is_missing(&error) && !required => Ok(None),
        Err(_) => Err(invalid(
            "Cannot inspect an application configuration file; selected profiles must exist.",
        )),
    }
}

fn is_missing(error: &std::io::Error) -> bool {
    matches!(error.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory)
}

fn open_regular_file(directory: &Dir, name: &str) -> Result<File, Failure> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No).nonblock(true);
    let file = directory
        .open_with(name, &options)
        .map_err(|_| invalid("Cannot open the application configuration file safely."))?;
    if !file.metadata().is_ok_and(|metadata| metadata.is_file()) {
        return Err(invalid(
            "Application configuration must remain a regular file.",
        ));
    }
    Ok(file)
}

fn read_limited(file: File) -> Result<Vec<u8>, Failure> {
    let limit = ApplicationConfig::MAX_BYTES as u64 + 1;
    let mut bytes = Vec::new();
    file.take(limit)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid("Cannot read the application configuration file."))?;
    if bytes.len() > ApplicationConfig::MAX_BYTES {
        return Err(invalid("Application configuration exceeds 64 KiB."));
    }
    Ok(bytes)
}

fn validate_profile(profile: &str) -> Result<(), Failure> {
    if profile.is_empty()
        || profile.len() > 32
        || !profile
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(invalid(
            "YON_PROFILE must contain 1-32 ASCII letters, digits, or hyphens.",
        ));
    }
    Ok(())
}

fn merge(target: &mut Value, incoming: Value) {
    if let (Value::Object(target), Value::Object(incoming)) = (&mut *target, &incoming) {
        for (key, value) in incoming {
            merge(
                target.entry(key.clone()).or_insert(Value::Null),
                value.clone(),
            );
        }
    } else {
        *target = incoming;
    }
}

fn bounded_value(value: String) -> Result<String, Failure> {
    if value.len() > ApplicationConfig::MAX_RESOLVED_BYTES || value.chars().any(char::is_control) {
        return Err(invalid(
            "Application values must contain no control characters and be at most 16 KiB.",
        ));
    }
    Ok(value)
}

fn parse_host(value: Option<&Value>) -> Result<Option<ServerHost>, Failure> {
    let Some(value) = value else {
        return Ok(None);
    };
    let message = "yon.server.host must be localhost, a name under .localhost or an IP address.";
    let value = value.as_str().ok_or_else(|| invalid(message))?;
    ServerHost::parse(value)
        .map(Some)
        .ok_or_else(|| invalid(message))
}

fn parse_port(value: Option<&Value>) -> Result<Option<u16>, Failure> {
    let Some(value) = value else {
        return Ok(None);
    };
    let port = match value {
        Value::String(value)
            if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            value.parse::<u16>().ok()
        }
        Value::Number(value) => value.as_u64().and_then(|number| u16::try_from(number).ok()),
        _ => None,
    };
    port.map(Some)
        .ok_or_else(|| invalid("yon.server.port must be an integer from 0 through 65535."))
}

fn normalize_endpoint(document: &mut Value, host: Option<&ServerHost>, port: Option<u16>) {
    if host.is_none() && port.is_none() {
        return;
    }
    let Some(root) = document.as_object_mut() else {
        return;
    };
    let Some(yon) = child_object(root, "yon") else {
        return;
    };
    let Some(server) = child_object(yon, "server") else {
        return;
    };
    if let Some(host) = host {
        server.insert("host".into(), Value::String(host.setting()));
    }
    if let Some(port) = port {
        server.insert("port".into(), Value::from(port));
    }
}

fn child_object<'a>(
    parent: &'a mut Map<String, Value>,
    key: &str,
) -> Option<&'a mut Map<String, Value>> {
    parent
        .entry(key)
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
}

fn invalid(message: &'static str) -> Failure {
    let help = String::from(
        "Use only tac/yon namespaces in application.json and profiles; put server secrets only in project-root .env using YON_ names. Underscores map to nested lowercase keys. Values are omitted from diagnostics.",
    );
    Failure::one(diagnostic(2010, message, Some(help), None))
}

fn read_environment(name: &str) -> Result<Option<String>, Failure> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(invalid(
            "A selected configuration environment value is not Unicode.",
        )),
    }
}
