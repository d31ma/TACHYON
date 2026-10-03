use super::ApplicationConfig;
use crate::Failure;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use tempfile::TempDir;

fn temporary_directory() -> TempDir {
    tempfile::tempdir().unwrap_or_else(|_| unreachable!())
}

fn create_directory(path: &Path) {
    std::fs::create_dir_all(path).unwrap_or_else(|_| unreachable!());
}

fn write(root: &Path, name: &str, source: &str) {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        create_directory(parent);
    }
    std::fs::write(path, source).unwrap_or_else(|_| unreachable!());
}

fn load(root: &Path) -> Result<ApplicationConfig, Failure> {
    ApplicationConfig::load_with_environment(root, &BTreeMap::new())
}

fn load_with(root: &Path, variables: &[(&str, &str)]) -> Result<ApplicationConfig, Failure> {
    let mut environment = BTreeMap::new();
    for (name, value) in variables {
        environment.insert(String::from(*name), String::from(*value));
    }
    ApplicationConfig::load_with_environment(root, &environment)
}

fn loaded(result: Result<ApplicationConfig, Failure>) -> ApplicationConfig {
    result.unwrap_or_else(|error| panic!("{error}"))
}

fn configured(source: &str, secrets: &str) -> Result<ApplicationConfig, Failure> {
    let root = temporary_directory();
    write(root.path(), "server/application.json", source);
    write(root.path(), ".env", secrets);
    load(root.path())
}

fn json(config: &ApplicationConfig) -> Value {
    serde_json::from_str(config.handler_json().unwrap_or("{}")).unwrap_or_else(|_| unreachable!())
}

fn object_with_keys(count: usize) -> String {
    let mut members = Vec::new();
    for index in 0..count {
        members.push(format!("\"key{index}\":0"));
    }
    format!("{{{}}}", members.join(","))
}

#[test]
fn missing_configuration_preserves_defaults() {
    let root = temporary_directory();
    let config = loaded(load(root.path()));
    assert_eq!(config, ApplicationConfig::default());
    assert_eq!(config.host(), None);
    assert_eq!(config.port(), None);
}

#[test]
fn profiles_merge_typed_configuration_before_secrets() {
    let root = temporary_directory();
    write(
        root.path(),
        "server/application.json",
        r#"{"server":{"host":"localhost","port":8080},"catalog":{"page":{"size":20},"enabled":true,"optional":null,"regions":["ca","us"]}}"#,
    );
    write(
        root.path(),
        "server/application-production.json",
        r#"{"server":{"port":9090},"catalog":{"page":{"size":30}}}"#,
    );
    write(
        root.path(),
        ".env",
        "YON_DATASOURCE_PASSWORD='private-canary'\nYON_AUTH_SIGNING_KEY=opaque",
    );
    let config = loaded(load_with(
        root.path(),
        &[
            ("YON_PROFILE", "production"),
            ("YON_PORT", "7070"),
            ("YON_CATALOG_PAGE_SIZE", "40"),
            ("YON_DATASOURCE_PASSWORD", "ignored"),
        ],
    ));
    assert_eq!(config.port(), Some(9090));
    assert_eq!(config.host(), Some(std::net::Ipv4Addr::LOCALHOST.into()));
    let snapshot = json(&config);
    assert_eq!(
        snapshot["yon"]["catalog"],
        json!({"page":{"size":30},"enabled":true,"optional":null,"regions":["ca","us"]})
    );
    assert_eq!(snapshot["yon"]["datasource"]["password"], "private-canary");
    assert_eq!(snapshot["yon"]["auth"]["signing"]["key"], "opaque");
}

#[test]
fn parent_environment_never_provides_configuration_or_secrets() {
    let root = temporary_directory();
    let config = loaded(load_with(
        root.path(),
        &[
            ("HOST", "bad"),
            ("PORT", "1"),
            ("YON_HOST", "bad"),
            ("YON_PORT", "2"),
            ("YON_DATASOURCE_PASSWORD", "private-canary"),
            ("YON_APPLICATION_JSON", "untrusted"),
        ],
    ));
    assert_eq!(config, ApplicationConfig::default());
}

#[test]
fn explicit_endpoint_overrides_supersede_invalid_file_endpoints() {
    let root = temporary_directory();
    write(
        root.path(),
        "server/application.json",
        r#"{"server":{"host":"not-an-ip","port":"not-a-port"}}"#,
    );
    let config = loaded(ApplicationConfig::load_with_endpoint_overrides(
        root.path(),
        &|_| Ok(None),
        crate::ServerHost::parse("127.0.0.1"),
        Some(0),
    ));
    assert_eq!(config.port(), Some(0));
    assert_eq!(json(&config)["yon"]["server"]["host"], "127.0.0.1");
}

#[test]
fn a_server_host_under_localhost_binds_loopback_and_keeps_its_name() {
    let config = loaded(configured(r#"{"server":{"host":"Shop.localhost"}}"#, ""));
    assert_eq!(config.host(), Some(std::net::Ipv4Addr::LOCALHOST.into()));
    assert_eq!(json(&config)["yon"]["server"]["host"], "shop.localhost");
    let root = temporary_directory();
    let explicit = loaded(ApplicationConfig::load_with_endpoint_overrides(
        root.path(),
        &|_| Ok(None),
        crate::ServerHost::parse("admin.localhost"),
        None,
    ));
    assert_eq!(json(&explicit)["yon"]["server"]["host"], "admin.localhost");
}

#[test]
fn dotenv_values_are_opaque_and_not_interpolated() {
    let secrets = "# comment\nYON_CUSTOM_VALUE='${YON_NEVER_EXPAND} $(no-command) #literal'\nYON_OTHER_VALUE=\"a=b\"\nYON_EMPTY=\n";
    let config = loaded(configured("{}", secrets));
    assert_eq!(
        json(&config)["yon"]["custom"]["value"],
        "${YON_NEVER_EXPAND} $(no-command) #literal"
    );
    assert_eq!(json(&config)["yon"]["other"]["value"], "a=b");
    assert_eq!(json(&config)["yon"]["empty"], "");
}

#[test]
fn malformed_or_reserved_dotenv_names_are_rejected() {
    for source in [
        "TAC_TOKEN=value",
        "HOME=value",
        "YON_=value",
        "YON_A__B=value",
        "YON_3VALUE=value",
        "YON_lower=value",
        "export YON_TOKEN=value",
        "YON_TOKEN",
        "YON_TOKEN='unclosed",
        "YON_TOKEN=x\nYON_TOKEN=y",
        "YON_APPLICATION_JSON=x",
        "YON_RESOURCE_ENDPOINT=x",
        "YON_RESOURCE_TOKEN=x",
        "YON_PROFILE=dev",
        "YON_SERVER_PORT=8000",
        "YON_SERVER=x",
        "YON_HOST=x",
        "YON_PORT=x",
        "YON_ISOLATION=process",
        "YON_FIRECRACKER_DRIVER=x",
        "YON_HANDLER_PROTOCOL=x",
        "YON_SKIP_BUNDLE=x",
        "YON_JAVASCRIPT_RUNTIME=x",
        "YON_PYTHON_RUNTIME=x",
        "YON_RUBY_RUNTIME=x",
        "YON_CONSTRUCTOR=x",
        "YON_A=x\u{0000}y",
    ] {
        assert!(configured("{}", source).is_err());
    }
}

#[test]
fn secret_collisions_are_rejected_in_both_path_orders() {
    for (source, secrets) in [
        (r#"{"custom":"public"}"#, "YON_CUSTOM=private"),
        (r#"{"custom":"public"}"#, "YON_CUSTOM_VALUE=private"),
        (r#"{"custom":{"value":"public"}}"#, "YON_CUSTOM=private"),
        ("{}", "YON_CUSTOM=private\nYON_CUSTOM_VALUE=private"),
        ("{}", "YON_CUSTOM_VALUE=private\nYON_CUSTOM=private"),
        (r#"{"custom":[]}"#, "YON_CUSTOM_VALUE=private"),
    ] {
        assert!(configured(source, secrets).is_err());
    }
}

#[test]
fn authoring_rejects_credential_fields() {
    for key in [
        "password",
        "passwd",
        "secret",
        "secrets",
        "token",
        "tokens",
        "credential",
        "credentials",
        "privatekey",
        "signingkey",
        "apikey",
        "accesskey",
        "clientsecret",
    ] {
        assert!(configured(&json!({key:null}).to_string(), "").is_err());
    }
    assert!(configured(r#"{"key":"lookup-key"}"#, "").is_ok());
}

#[test]
fn authoring_errors_for_credentials_and_placeholders_omit_the_value() {
    for source in [
        r#"{"datasource":{"password":"private-canary"}}"#,
        r#"{"tokens":"private-canary"}"#,
        r#"{"auth":{"signing":{"key":"private-canary"}}}"#,
        r#"{"items":[{"clientsecret":"private-canary"}]}"#,
        r#"{"url":"postgres://user:private-canary@host/db"}"#,
        r#"{"value":"${YON_TOKEN}"}"#,
    ] {
        let error = configured(source, "")
            .err()
            .unwrap_or_else(|| unreachable!());
        assert!(!error.to_string().contains("private-canary"));
    }
}

#[test]
fn v3_schema_examples_match_the_runtime_authoring_boundary() {
    let valid = include_str!("../../../../api/application-config/v3/examples/valid.json");
    let invalid = include_str!("../../../../api/application-config/v3/examples/invalid.json");
    assert!(configured(valid, "").is_ok());
    assert!(configured(invalid, "").is_err());
}

#[test]
fn malformed_or_unsafe_configuration_is_rejected() {
    for source in [
        "[]",
        "null",
        "{} true",
        r#"{"yon":{}}"#,
        r#"{"tac":{}}"#,
        r#"{"server":{},"server":{}}"#,
        r#"{"x":1,"x":2}"#,
        r#"{"__proto__":{}}"#,
        r#"{"constructor":{}}"#,
        r#"{"page_size":1}"#,
        r#"{"Page":1}"#,
        r#"{"page.size":1}"#,
        "null",
        r#"{"server":{"port":65536}}"#,
        r#"{"server":{"port":null}}"#,
        r#"{"server":{"host":false}}"#,
        r#"{"server":{"port":"-1"}}"#,
        r#"{"server":{"host":"example.com"}}"#,
        r#"{"server":{"typo":true}}"#,
        r#"{"value":"line\nfeed"}"#,
    ] {
        assert!(configured(source, "").is_err());
    }
}

#[test]
fn profiles_are_optional_until_selected_and_paths_are_bounded() {
    let root = temporary_directory();
    write(
        root.path(),
        "server/application-development.json",
        "invalid",
    );
    assert!(load(root.path()).is_ok());
    for profile in [
        "",
        "../secret",
        "dev/prod",
        "dev,prod",
        "dev_prod",
        "missing",
    ] {
        assert!(load_with(root.path(), &[("YON_PROFILE", profile)]).is_err());
    }
}

#[test]
fn source_and_resolved_output_are_bounded() {
    assert!(configured(&" ".repeat(65_537), "").is_err());
    assert!(configured("{}", &format!("YON_TOKEN={}", "x".repeat(16_385))).is_err());
    let deep = format!("{}0{}", "[".repeat(17), "]".repeat(17));
    assert!(configured(&deep, "").is_err());
    assert!(configured(&object_with_keys(129), "").is_err());
    assert!(configured(&object_with_keys(126), "YON_AUTH_TOKEN=value").is_err());
    let source = json!({"label":"x".repeat(8192)}).to_string();
    assert!(configured(&source, &format!("YON_TOKEN={}", "x".repeat(8192))).is_err());
}

#[test]
fn loaded_configuration_is_an_owned_redacted_snapshot() {
    let root = temporary_directory();
    write(
        root.path(),
        "server/application.json",
        r#"{"label":"first"}"#,
    );
    write(root.path(), ".env", "YON_TOKEN=private-canary");
    let config = loaded(load(root.path()));
    write(root.path(), "server/application.json", "{}");
    write(root.path(), ".env", "YON_TOKEN=changed");
    assert_eq!(json(&config)["yon"]["label"], "first");
    assert_eq!(json(&config)["yon"]["token"], "private-canary");
    assert!(!format!("{config:?}").contains("private-canary"));
    assert_eq!(config.clone(), config);
}

#[test]
fn client_projection_never_opens_secrets_or_profiles() {
    let root = temporary_directory();
    write(
        root.path(),
        "client/manifest.json",
        r#"{"name":"Catalog","short_name":"Catalog","icons":[{"src":"/icon.svg"}],"tac":{"api":{"url":"/api"}}}"#,
    );
    write(
        root.path(),
        "server/application.json",
        r#"{"label":"private"}"#,
    );
    create_directory(&root.path().join(".env"));
    write(
        root.path(),
        "server/application-development.json",
        "invalid",
    );
    let config = loaded(ApplicationConfig::load_client(root.path()));
    assert_eq!(json(&config), json!({"tac":{"api":{"url":"/api"}}}));
    write(
        root.path(),
        "server/application.json",
        r#"{"tac":{"api":{"url":"/api"}},"yon":{}}"#,
    );
    assert!(ApplicationConfig::load_client(root.path()).is_err());
    assert!(ApplicationConfig::load(root.path()).is_err());
}

#[cfg(unix)]
#[test]
fn retained_capability_reads_original_configuration_and_secrets() {
    let temporary = temporary_directory();
    let root = temporary.path().join("project");
    create_directory(&root.join("client/pages"));
    write(&root, "client/pages/tac.html", "<main>Public</main>");
    write(&root, "server/application.json", r#"{"name":"original"}"#);
    write(&root, ".env", "YON_CUSTOM=private-original");
    let project =
        crate::ProjectDiscovery::discover(&root).unwrap_or_else(|error| panic!("{error}"));
    std::fs::rename(&root, temporary.path().join("original")).unwrap_or_else(|_| unreachable!());
    create_directory(&root);
    write(
        &root,
        "server/application.json",
        r#"{"name":"replacement"}"#,
    );
    write(&root, ".env", "YON_CUSTOM=private-replacement");
    let config = loaded(ApplicationConfig::load_opened(
        &project.capability(),
        &|_| Ok(None),
        None,
        None,
    ));
    assert_eq!(json(&config)["yon"]["name"], "original");
    assert_eq!(json(&config)["yon"]["custom"], "private-original");
}

#[test]
fn a_root_tachyon_json_is_refused_and_names_the_manifest() {
    let root = temporary_directory();
    write(
        root.path(),
        "tachyon.json",
        r#"{"application":{"name":"App","id":"dev.example.app","version":"1","entry_route":"/"}}"#,
    );
    for result in [
        ApplicationConfig::load_client(root.path()),
        load(root.path()),
    ] {
        let Err(failure) = result else {
            panic!("tachyon.json was accepted");
        };
        let text = failure.to_string();
        assert!(
            text.contains("client/manifest.json under tac.application"),
            "{text}"
        );
    }
}

#[test]
fn every_tac_config_file_is_refused_and_names_the_manifest() {
    for name in ["tac.config.js", "tac.config.mjs", "tac.config.ts"] {
        let root = temporary_directory();
        write(
            root.path(),
            name,
            "export default class Configuration { static scripts = [] }",
        );
        for result in [
            ApplicationConfig::load_client(root.path()),
            load(root.path()),
        ] {
            let Err(failure) = result else {
                panic!("{name} was accepted");
            };
            let text = failure.to_string();
            assert!(text.contains("tac.config is no longer read"), "{text}");
            assert!(text.contains("tac.metadata.routes"), "{text}");
            assert!(text.contains("tac.sitemap"), "{text}");
        }
    }
}

#[test]
fn tachyon_settings_in_the_tac_member_are_not_service_configuration() {
    let root = temporary_directory();
    write(
        root.path(),
        "client/manifest.json",
        r#"{"tac":{"api":{"url":"/api"},"origin":"https://shop.example","metadata":{"routes":{"/docs/_topic":{"siteName":"Shop"}}},"application":{"id":"dev.example.shop","window":{"minWidth":320}}}}"#,
    );
    let config = loaded(ApplicationConfig::load_client(root.path()));
    assert_eq!(json(&config), json!({"tac":{"api":{"url":"/api"}}}));
    write(
        root.path(),
        "client/manifest.json",
        r#"{"tac":{"apiUrl":"/api"}}"#,
    );
    assert!(ApplicationConfig::load_client(root.path()).is_err());
}

#[cfg(unix)]
#[test]
fn symlinks_sockets_and_directories_are_rejected_for_both_private_files() {
    for name in ["server/application.json", ".env"] {
        for destination in ["missing", "real"] {
            let root = temporary_directory();
            write(root.path(), "real", "{}");
            create_directory(&root.path().join("server"));
            std::os::unix::fs::symlink(destination, root.path().join(name))
                .unwrap_or_else(|_| unreachable!());
            assert!(load(root.path()).is_err());
        }
        let root = temporary_directory();
        create_directory(&root.path().join("server"));
        let _socket = std::os::unix::net::UnixListener::bind(root.path().join(name))
            .unwrap_or_else(|_| unreachable!());
        assert!(load(root.path()).is_err());
        let root = temporary_directory();
        create_directory(&root.path().join("server"));
        create_directory(&root.path().join(name));
        assert!(load(root.path()).is_err());
    }
}

#[test]
fn the_topic_log_folder_is_read_as_text() {
    let unset = loaded(configured("{}", ""));
    assert_eq!(unset.topics_directory(), None);
    let set = loaded(configured(
        r#"{"topics":{"directory":"storage/events"}}"#,
        "",
    ));
    assert_eq!(set.topics_directory(), Some("storage/events"));
    assert!(configured(r#"{"topics":{"directory":7}}"#, "").is_err());
}
