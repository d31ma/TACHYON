use super::name_refusal::NameRefusal;
use super::{ApplicationConfig, Failure, bounded_value, invalid, read_bytes};
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
    NameRefusal::of(name).map_or_else(
        || Ok(NameRefusal::segments(name)),
        |refusal| Err(invalid(refusal.message())),
    )
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
