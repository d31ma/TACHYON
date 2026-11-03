use super::{Input, Method, Registration};
use crate::Failure;
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn resolve(
    input: &Input<'_>,
    registrations: &mut [Registration],
) -> Result<(), Failure> {
    let entities = entity_tables(registrations);
    for registration in registrations {
        let Some(repository) = registration.data.get("repository").cloned() else {
            reject_queries_without_repository(input, registration)?;
            continue;
        };
        let entity_name = repository["entity"].as_str().unwrap_or_default();
        let (entity, table) = entities.get(entity_name).ok_or_else(|| {
            input.failure(
                registration.start,
                "Store must bind a source-local Entity/Table class",
            )
        })?;
        if !repository["fields"]
            .as_array()
            .is_some_and(|fields| fields.contains(&entity["id"]))
        {
            return Err(input.failure(
                registration.start,
                "Store.fields must include its Entity id field",
            ));
        }
        for start in method_starts(input, registration) {
            resolve_query_method(
                input,
                registration,
                start,
                &repository,
                entity_name,
                entity,
                table,
            )?;
        }
    }
    Ok(())
}

fn entity_tables(registrations: &[Registration]) -> BTreeMap<String, (Value, String)> {
    let mut entities = BTreeMap::new();
    for registration in registrations {
        let Some(entity) = registration.data.get("entity") else {
            continue;
        };
        let Some(table) = registration.data.get("table").and_then(Value::as_str) else {
            continue;
        };
        entities.insert(
            registration.name.clone(),
            (entity.clone(), table.to_owned()),
        );
    }
    entities
}

fn reject_queries_without_repository(
    input: &Input<'_>,
    registration: &Registration,
) -> Result<(), Failure> {
    if registration
        .methods
        .values()
        .any(|method| method.contains_key("query"))
    {
        return Err(input.failure(
            registration.start,
            "Query requires an entity-bound Store class",
        ));
    }
    Ok(())
}

fn resolve_query_method(
    input: &Input<'_>,
    registration: &mut Registration,
    start: usize,
    repository: &Value,
    entity_name: &str,
    entity: &Value,
    table: &str,
) -> Result<(), Failure> {
    let method = super::method(input, registration, start)?;
    let page = registration
        .methods
        .get_mut(&method.name)
        .and_then(|metadata| metadata.remove("pageParameter"));
    let metadata = registration.methods.get(&method.name);
    let explicit = metadata.and_then(|metadata| metadata.get("query")).cloned();
    if explicit.is_none() && !super::queries::derives_name(&method.name) {
        if let Some(page) = page {
            registration
                .methods
                .entry(method.name)
                .or_default()
                .insert("pageParameter".into(), page);
        }
        return Ok(());
    }
    if metadata.is_some_and(|metadata| metadata.keys().any(|key| key != "query")) {
        return Err(input.failure(
            start,
            "Query methods cannot combine other behavior method annotations",
        ));
    }
    validate_body(input, &method, start)?;
    super::concrete_member(registration, &method);
    let arity = arity(input, &method, start)?;
    let query = super::queries::resolve(
        input,
        start,
        repository,
        entity_name,
        entity,
        table,
        &method.name,
        arity,
        explicit.as_ref(),
        page.as_ref(),
    )?;
    registration
        .methods
        .entry(method.name)
        .or_default()
        .insert("query".into(), query);
    Ok(())
}

fn validate_body(input: &Input<'_>, method: &Method, at: usize) -> Result<(), Failure> {
    if method.declared {
        if method.is_static {
            return Err(input.failure(at, "Store query methods require instance methods"));
        }
        return Ok(());
    }
    if method.is_static
        || !method.asynchronous
        || !input.source[method.body.clone()].trim().is_empty()
    {
        return Err(input.failure(at, "Store query methods require async instance methods with empty generated bodies; application logic belongs in a service"));
    }
    Ok(())
}

fn arity(input: &Input<'_>, method: &Method, at: usize) -> Result<usize, Failure> {
    let parameters = super::polyglot::split_parameters(&input.code[method.parameters.clone()]);
    if parameters.len() > 32
        || !parameters
            .iter()
            .all(|parameter| is_plain_parameter(parameter))
    {
        return Err(input.failure(at, "Query parameters require at most 32 named positional arguments without defaults, optional/rest or destructuring syntax"));
    }
    Ok(parameters.len())
}

fn is_plain_parameter(parameter: &str) -> bool {
    let name = parameter.split(':').next().unwrap_or("").trim();
    super::identifier(name) && !parameter.contains(['=', '?'])
}

fn method_starts(input: &Input<'_>, registration: &Registration) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut depth = 0_u32;
    let mut start = registration.open + 1;
    for (at, byte) in input
        .code
        .bytes()
        .enumerate()
        .take(registration.close)
        .skip(start)
    {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    start = at + 1;
                }
            }
            b';' | b'\n' if depth == 0 => start = at + 1,
            b'(' if depth == 0 => {
                let prefix = &input.code[start..at];
                if is_method_declaration_prefix(prefix, registration.declared) {
                    starts.push(start + prefix.len() - prefix.trim_start().len());
                }
            }
            _ => {}
        }
    }
    starts
}

fn is_method_declaration_prefix(prefix: &str, declared: bool) -> bool {
    let words = prefix.split_whitespace().collect::<Vec<_>>();
    let Some((name, modifiers)) = words.split_last() else {
        return false;
    };
    super::identifier(name)
        && modifiers
            .iter()
            .all(|word| is_query_method_modifier(word, declared))
}

fn is_query_method_modifier(word: &str, declared: bool) -> bool {
    matches!(
        word,
        "async" | "static" | "public" | "private" | "protected"
    ) || (word == "abstract" && declared)
}
