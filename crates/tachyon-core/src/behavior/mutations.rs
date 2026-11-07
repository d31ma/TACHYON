use super::{Input, Marker};
use crate::Failure;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub(super) fn policy(input: &Input<'_>, marker: &Marker) -> Result<Value, Failure> {
    let written = super::literal::parse(&marker.arguments).ok();
    if written
        .as_ref()
        .is_some_and(|value| value.get("arguments").is_some())
    {
        return Err(input.failure(
            marker.span.start,
            format!(
                "{} matches parameters by name now: remove \"arguments\" and give this method parameters with the same names as the Cached method's key parameters",
                marker.name
            ),
        ));
    }
    let object = super::object(input, marker, &["method"])?;
    super::named(input, marker, &object, "method")?;
    Ok(Value::Object(object))
}

pub(super) fn uses_parameter_names(key: &str) -> bool {
    matches!(key, "cacheable" | "cacheEvict" | "cachePut")
}

pub(super) fn resolve(
    input: &Input<'_>,
    at: usize,
    methods: &mut BTreeMap<String, Map<String, Value>>,
) -> Result<(), Failure> {
    let snapshot = methods.clone();
    for (name, method) in methods.iter_mut() {
        let operations = ["cacheEvict", "cachePut"]
            .into_iter()
            .filter(|key| method.contains_key(*key))
            .collect::<Vec<_>>();
        if operations.is_empty() {
            continue;
        }
        if operations.len() != 1
            || method.contains_key("pageable")
            || method.contains_key("cacheable")
            || super::Method::HTTP_VERBS.contains(&name.as_str())
        {
            return Err(input.failure(at, "Cache mutation methods cannot combine Cached, UpdateCache and ClearCache or wrap HTTP verb endpoints"));
        }
        let own_names = names(method);
        let policy = &mut method[operations[0]];
        let (arguments, target) = resolved_target(input, at, &snapshot, policy, &own_names)?;
        policy["arguments"] = arguments;
        policy["target"] = target;
    }
    for method in methods.values_mut() {
        method.remove("parameterNames");
    }
    Ok(())
}

fn names(method: &Map<String, Value>) -> Vec<String> {
    let mut names = Vec::new();
    let Some(Value::Array(written)) = method.get("parameterNames") else {
        return names;
    };
    for name in written {
        names.push(name.as_str().unwrap_or_default().to_owned());
    }
    names
}

fn resolved_target(
    input: &Input<'_>,
    at: usize,
    methods: &BTreeMap<String, Map<String, Value>>,
    policy: &Value,
    own_names: &[String],
) -> Result<(Value, Value), Failure> {
    let target_name = policy["method"].as_str().unwrap_or_default().to_owned();
    let Some(target) = methods
        .get(&target_name)
        .filter(|target| target.contains_key("cacheable"))
    else {
        return Err(input.failure(
            at,
            "Cache mutation target must name a Cached method in the same class",
        ));
    };
    let needed = needed_positions(target);
    let target_names = names(target);
    let mut arguments = Vec::new();
    let count = needed.iter().max().map_or(0, |last| last + 1);
    for position in 0..count {
        let target_parameter = target_names.get(position).cloned().unwrap_or_default();
        let own = own_names.iter().position(|name| *name == target_parameter);
        match own {
            Some(index) => arguments.push(Value::from(index)),
            None if needed.contains(&position) => {
                return Err(input.failure(
                    at,
                    format!(
                        "This cache mutation needs a parameter named '{target_parameter}', because the Cached method {target_name} uses it in its cache key"
                    ),
                ));
            }
            None => arguments.push(Value::from(0)),
        }
    }
    let mut resolved = Map::new();
    resolved.insert("method".into(), Value::String(target_name));
    resolved.insert("cacheable".into(), target["cacheable"].clone());
    if let Some(page) = target.get("pageable") {
        resolved.insert("pageable".into(), page.clone());
    }
    Ok((Value::Array(arguments), Value::Object(resolved)))
}

fn needed_positions(target: &Map<String, Value>) -> Vec<usize> {
    let mut needed = Vec::new();
    if let Some(Value::Array(keys)) = target["cacheable"].get("keyArgs") {
        for key in keys {
            needed.extend(key.as_u64().and_then(|key| usize::try_from(key).ok()));
        }
    }
    if let Some(page) = target.get("pageable") {
        needed.extend(
            page["argument"]
                .as_u64()
                .and_then(|key| usize::try_from(key).ok()),
        );
    }
    needed
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use std::path::Path;

    #[test]
    fn lowers_targeted_cache_mutations() {
        for annotation in ["UpdateCache", "ClearCache"] {
            let source = format!(
                "@CacheSettings({{\"name\":\"items\",\"ttlMs\":1000,\"maxBytes\":1024,\"maxEntries\":8}})\nclass Items {{\n@Cached\nasync find(@CacheKey id, view) {{ return id; }}\n@{annotation}({{\"method\":\"find\"}})\nasync update(value, id) {{ return value; }}\n}}"
            );
            let output = super::super::lower(Path::new("yon.js"), &source, "yon")
                .unwrap()
                .unwrap();
            assert!(!output.contains(&format!("@{annotation}")));
            assert!(output.contains("\"target\""));
            assert!(output.contains("\"arguments\":[1]"), "{output}");
            assert!(output.contains("\"keyArgs\":[0]"), "{output}");
            assert!(!output.contains("parameterNames"), "{output}");
        }
    }

    #[test]
    fn a_mutation_needs_the_key_parameter_by_name_and_refuses_indexes() {
        let source = |cached: &str, mutation: &str| {
            format!(
                "@CacheSettings({{\"name\":\"items\",\"ttlMs\":1000,\"maxBytes\":1024,\"maxEntries\":8}})\nclass Items {{\n{cached}\nasync find(id) {{ return id; }}\n{mutation}\nasync update(key, value) {{ return value; }}\n}}"
            )
        };
        let missing = source("@Cached", "@ClearCache({\"method\":\"find\"})");
        let failure = super::super::lower(Path::new("yon.js"), &missing, "yon").unwrap_err();
        assert!(failure.to_string().contains("named 'id'"), "{failure}");
        let indexes = source(
            "@Cached",
            "@ClearCache({\"method\":\"find\",\"arguments\":[0]})",
        );
        let failure = super::super::lower(Path::new("yon.js"), &indexes, "yon").unwrap_err();
        assert!(failure.to_string().contains("by name"), "{failure}");
        let old_cached = source("@Cached({\"keyArgs\":[0]})", "");
        let failure = super::super::lower(Path::new("yon.js"), &old_cached, "yon").unwrap_err();
        assert!(failure.to_string().contains("@CacheKey"), "{failure}");
    }
}
