use super::{Input, Method, Type};
use crate::Failure;
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn resolve(
    input: &Input<'_>,
    language: &str,
    types: &mut [Type],
) -> Result<(), Failure> {
    let entities = entity_tables(types);
    discover(input, language, types)?;
    for ty in types {
        let Some(repository) = &ty.repository else {
            reject_queries_without_repository(input, ty)?;
            continue;
        };
        let entity_name = repository["entity"].as_str().unwrap_or_default();
        let (entity, table) = entities.get(entity_name).ok_or_else(|| {
            input.failure(
                ty.declaration,
                "Store requires a source-local Entity/Table type",
            )
        })?;
        if !repository["fields"]
            .as_array()
            .is_some_and(|fields| fields.contains(&entity["id"]))
        {
            return Err(input.failure(
                ty.declaration,
                "Store.fields must include its Entity id field",
            ));
        }
        for method in &mut ty.methods {
            resolve_query_method(
                input,
                language,
                method,
                repository,
                entity_name,
                entity,
                table,
            )?;
        }
    }
    Ok(())
}

fn entity_tables(types: &[Type]) -> BTreeMap<String, (Value, String)> {
    let mut entities = BTreeMap::new();
    for ty in types {
        let (Some(entity), Some(table)) = (&ty.entity, &ty.table) else {
            continue;
        };
        entities.insert(ty.name.clone(), (entity.clone(), table.clone()));
    }
    entities
}

fn reject_queries_without_repository(input: &Input<'_>, ty: &Type) -> Result<(), Failure> {
    if ty
        .methods
        .iter()
        .any(|method| method.metadata.contains_key("query"))
    {
        return Err(input.failure(ty.declaration, "Query requires an entity-bound Store type"));
    }
    Ok(())
}

fn resolve_query_method(
    input: &Input<'_>,
    language: &str,
    method: &mut Method,
    repository: &Value,
    entity_name: &str,
    entity: &Value,
    table: &str,
) -> Result<(), Failure> {
    let name = query_name(language, &method.name);
    if !method.metadata.contains_key("query") && !super::super::queries::derives_name(&name) {
        return Ok(());
    }
    let page = method.metadata.remove("pageParameter");
    let query = method.metadata.get("query");
    if method.metadata.keys().any(|key| key != "query") {
        return Err(input.failure(
            method.signature.start,
            "Query methods cannot combine other behavior method annotations",
        ));
    }
    if method.is_static || !super::has_empty_body(input, language, method) {
        return Err(input.failure(method.signature.start, "Query methods require empty generated instance bodies (Python: pass); put application logic in a service"));
    }
    let resolved = super::super::queries::resolve(
        input,
        method.signature.start,
        repository,
        entity_name,
        entity,
        table,
        &name,
        method.parameters.len(),
        query,
        page.as_ref(),
    )?;
    method.metadata.insert("query".into(), resolved);
    Ok(())
}

fn discover(input: &Input<'_>, language: &str, types: &mut [Type]) -> Result<(), Failure> {
    for (at, name) in super::tokens(&input.code) {
        if !super::super::queries::derives_name(&query_name(language, name))
            || !input.code[at + name.len()..].trim_start().starts_with('(')
        {
            continue;
        }
        let start = super::line_declaration(&input.code, at);
        if !declaration_prefix(&input.code[start..at], language) {
            continue;
        }
        let Some(owner) = super::method_owner(input, language, types, start) else {
            continue;
        };
        let ty = &mut types[owner];
        if ty.repository.is_none()
            || ty
                .methods
                .iter()
                .any(|method| method.signature.start == start)
        {
            continue;
        }
        let method = super::parse_method(input, language, start, ty)?;
        if ty
            .methods
            .iter()
            .any(|existing| existing.name == method.name)
        {
            return Err(input.failure(start, "Overloaded repository query methods are unsupported"));
        }
        ty.methods.push(method);
    }
    Ok(())
}

fn declaration_prefix(prefix: &str, language: &str) -> bool {
    if prefix.contains(['=', ';', '{', '}', '@', '#']) {
        return false;
    }
    let words = prefix.split_whitespace().collect::<Vec<_>>();
    match language {
        "py" | "rb" => words.last() == Some(&"def"),
        "kt" => words.last() == Some(&"fun"),
        "swift" => words.last() == Some(&"func"),
        "rs" => words.last() == Some(&"fn"),
        "php" => words.last() == Some(&"function"),
        "go" => {
            words.is_empty() || (words.first() == Some(&"func") && prefix.trim_end().ends_with(')'))
        }
        _ => !words.is_empty() && !prefix.trim_end().ends_with('.'),
    }
}

fn query_name(language: &str, name: &str) -> String {
    if language == "go" {
        return crate::go_source::lower_first(name);
    }
    name.to_owned()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    #[test]
    fn a_comment_after_a_derived_body_does_not_occupy_it() {
        let source = "@entity({\"adapter\":\"ledger\"})\n@table(\"entries\")\nclass EntryEntity:\n    @id()\n    id: int = 0\n\n@store(EntryEntity, {\"fields\":[\"id\",\"status\"],\"defaultSize\":20,\"maxSize\":100})\nclass EntryRepository:\n    behaviors: object\n    async def count_by_status(self, status):\n        pass\n\n    # Both derived queries run on one transaction.\n    @all_or_nothing({\"adapter\":\"ledger\"})\n    async def settle(self, status):\n        return await self.count_by_status(status)\n";
        let lowered = crate::behavior::lower(std::path::Path::new("yon.py"), source, "yon")
            .unwrap_or_else(|failure| panic!("{failure:?}"))
            .expect("lowered");
        assert!(lowered.contains("transactional"), "{lowered}");
    }

    #[test]
    fn python_body_ends_at_real_masked_decorators_not_string_or_comment_text() {
        let source = "class Products:\n    async def findById(self, id):\n        text = \"\"\"\n@not_a_decorator()\n\"\"\"\n# @also_not_a_decorator()\n        return text\n    @Query()\n    async def selected(self, id):\n        pass\n";
        let visibility = crate::lexical::code_mask("py", source).code;
        let code = super::super::visible_code(source, visibility);
        let decorator = source.find("@Query()").unwrap_or_else(|| unreachable!());
        let mut input = super::Input {
            path: std::path::Path::new("queries.py"),
            source,
            code,
        };
        let markers = super::super::annotations(&input, "py", "yon")
            .unwrap_or_else(|failure| panic!("Python markers failed: {failure:?}"));
        super::super::mask_annotations(&mut input, &markers, "py");
        let method = source.find("async def").unwrap_or_else(|| unreachable!());
        let body = super::super::python_body(&input, method)
            .unwrap_or_else(|failure| panic!("Python body failed: {failure:?}"));
        assert_eq!(body.end, decorator - 4);
        assert!(source[body].contains("return text"));
    }
}
