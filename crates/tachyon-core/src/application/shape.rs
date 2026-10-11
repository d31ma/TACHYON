use super::{Failure, bounded_value, invalid};
use serde_json::{Map, Value};

pub(super) fn validate(document: &Value) -> Result<(), Failure> {
    let root = document
        .as_object()
        .ok_or_else(|| invalid("Application configuration must be a JSON object."))?;
    for (namespace, value) in root {
        if !matches!(namespace.as_str(), "tac" | "yon") {
            return Err(invalid(
                "application.json has one root namespace, yon, and Tac's public configuration is the tac member of manifest.json; move old server/datasource/properties settings under yon.",
            ));
        }
        if !value.is_object() {
            return Err(invalid("The tac and yon namespaces must be JSON objects."));
        }
        visit(value, &mut vec![namespace.as_str()])?;
    }
    if let Some(server) = document.pointer("/yon/server") {
        validate_server(server)?;
    }
    Ok(())
}

fn validate_server(server: &Value) -> Result<(), Failure> {
    let server = server.as_object().ok_or_else(|| {
        invalid(
            "yon.server must be an object containing host, port, hosts, proxies, environment or maxbody.",
        )
    })?;
    let has_other_key = server.keys().any(|key| {
        !matches!(
            key.as_str(),
            "host" | "port" | "hosts" | "proxies" | "environment" | "maxbody"
        )
    });
    if has_other_key {
        return Err(invalid(
            "yon.server supports only host, port, hosts, proxies, environment and maxbody.",
        ));
    }
    let host_is_invalid = server.get("host").is_some_and(|value| !value.is_string());
    let port_is_invalid = server
        .get("port")
        .is_some_and(|value| !value.is_string() && !value.is_number());
    if host_is_invalid || port_is_invalid {
        return Err(invalid(
            "yon.server.host must be a string and yon.server.port must be a number or decimal string.",
        ));
    }
    Ok(())
}

fn visit<'a>(value: &'a Value, path: &mut Vec<&'a str>) -> Result<(), Failure> {
    match value {
        Value::Object(object) => visit_object(object, path),
        Value::Array(values) => {
            for value in values {
                visit(value, path)?;
            }
            Ok(())
        }
        Value::String(text) => check_string(text),
        _ => Ok(()),
    }
}

fn visit_object<'a>(
    object: &'a Map<String, Value>,
    path: &mut Vec<&'a str>,
) -> Result<(), Failure> {
    for (key, value) in object {
        if !is_path_segment(key) {
            return Err(invalid(
                "Configuration path segments must begin with a lowercase ASCII letter and contain only lowercase letters or digits; underscores and dots are path separators, not JSON keys.",
            ));
        }
        path.push(key);
        if is_credential_path(path) {
            return Err(invalid(
                "Credential fields cannot appear in application.json or profiles; move their values to project-root .env.",
            ));
        }
        visit(value, path)?;
        path.pop();
    }
    Ok(())
}

fn check_string(text: &str) -> Result<(), Failure> {
    bounded_value(text.to_owned())?;
    if text.contains("${") {
        return Err(invalid(
            "Environment placeholders are not allowed in application.json; configuration is literal and secrets belong in .env.",
        ));
    }
    if has_credential_in_url(text) {
        return Err(invalid(
            "Credential-bearing URLs belong in project-root .env, not application.json.",
        ));
    }
    Ok(())
}

fn has_credential_in_url(text: &str) -> bool {
    let Some((_, after_scheme)) = text.split_once("://") else {
        return false;
    };
    let authority = after_scheme.split('/').next().unwrap_or_default();
    authority.contains('@')
}

pub(super) fn is_path_segment(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 128
        && key.as_bytes()[0].is_ascii_lowercase()
        && key
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && !matches!(key, "prototype" | "constructor")
}

fn is_credential_path(path: &[&str]) -> bool {
    let key = path.last().copied().unwrap_or_default();
    let is_credential_name = matches!(
        key,
        "password"
            | "passwd"
            | "secret"
            | "secrets"
            | "token"
            | "tokens"
            | "credential"
            | "credentials"
            | "privatekey"
            | "signingkey"
            | "apikey"
            | "accesskey"
            | "clientsecret"
    );
    if is_credential_name {
        return true;
    }
    if key != "key" || path.len() < 2 {
        return false;
    }
    matches!(
        path[path.len() - 2],
        "private" | "signing" | "api" | "access"
    )
}
