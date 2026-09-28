use super::{ApplicationConfig, Failure, bounded_value, invalid, read_bytes, shape};
use cap_std::fs::Dir;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub(super) fn merge_file(directory: &Dir, document: &mut Value) -> Result<(), Failure> {
    let Some(bytes) = read_bytes(directory, ".env", false)? else {
        return Ok(());
    };
    let source =
        std::str::from_utf8(&bytes).map_err(|_| invalid("Project-root .env must be UTF-8."))?;
    let mut names = BTreeSet::new();
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, raw_value)) = line.split_once('=') else {
            return Err(invalid(
                "Each .env entry must be NAME=value; shell commands and export statements are not supported.",
            ));
        };
        let name = name.trim();
        let path = secret_path(name)?;
        if !names.insert(name) || names.len() > ApplicationConfig::MAX_KEYS {
            return Err(invalid(
                "Project-root .env has duplicate names or more than 128 entries.",
            ));
        }
        let value = unquote_value(raw_value.trim())?;
        insert_secret(document, &path, Value::String(value))?;
    }
    Ok(())
}

fn secret_path(name: &str) -> Result<Vec<String>, Failure> {
    if !is_secret_name(name) {
        return Err(invalid(
            "Project-root .env accepts only uppercase YON_ secret names; TAC_ secrets and non-prefixed names are forbidden.",
        ));
    }
    if is_reserved_name(name) {
        return Err(invalid(
            "Project-root .env cannot set framework transport, profile, endpoint, runtime, or isolation policy names.",
        ));
    }
    let segments: Vec<String> = name.split('_').map(str::to_ascii_lowercase).collect();
    if segments.len() > 16
        || segments
            .iter()
            .any(|segment| !shape::is_path_segment(segment))
    {
        return Err(invalid(
            "Secret names require nonempty underscore-separated segments beginning with ASCII letters, with at most 16 segments.",
        ));
    }
    Ok(segments)
}

fn is_secret_name(name: &str) -> bool {
    let allowed_bytes = name
        .bytes()
        .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_');
    name.starts_with("YON_") && name.len() <= 128 && allowed_bytes
}

fn is_reserved_name(name: &str) -> bool {
    matches!(
        name,
        "YON_PROFILE"
            | "YON_HOST"
            | "YON_HOSTNAME"
            | "YON_PORT"
            | "YON_ISOLATION"
            | "YON_SKIP_BUNDLE"
            | "YON_APPLICATION_JSON"
            | "YON_RESOURCE_ENDPOINT"
            | "YON_RESOURCE_TOKEN"
            | "YON_HANDLER_PROTOCOL"
            | "YON_JAVASCRIPT_RUNTIME"
            | "YON_PYTHON_RUNTIME"
            | "YON_RUBY_RUNTIME"
            | "YON_SERVER"
    ) || name.starts_with("YON_FIRECRACKER_")
        || name.starts_with("YON_SERVER_")
}

fn unquote_value(raw_value: &str) -> Result<String, Failure> {
    if !raw_value.starts_with(['\'', '"']) {
        return bounded_value(raw_value.to_owned());
    }
    let bytes = raw_value.as_bytes();
    if bytes.len() < 2 || bytes.first() != bytes.last() {
        return Err(invalid(
            "Quoted .env values must end with the same quote on one line.",
        ));
    }
    bounded_value(raw_value[1..raw_value.len() - 1].to_owned())
}

fn insert_secret(target: &mut Value, path: &[String], value: Value) -> Result<(), Failure> {
    let object = target
        .as_object_mut()
        .ok_or_else(|| invalid("Secret paths cannot replace configuration scalars or arrays."))?;
    let Some((first, rest)) = path.split_first() else {
        return Err(invalid("Secret paths cannot be empty."));
    };
    if rest.is_empty() {
        if object.contains_key(first) {
            return Err(invalid(
                "A secret path conflicts with an existing configuration or secret path; remove the duplicate instead of overriding it.",
            ));
        }
        object.insert(first.clone(), value);
        return Ok(());
    }
    let child = object
        .entry(first)
        .or_insert_with(|| Value::Object(Map::new()));
    insert_secret(child, rest, value)
}
