use super::{
    block_named, camel_case, declared_service, rust_field, rust_field_declarations, rust_read,
    rust_write, unnamed_service,
};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::unicode_name::UnicodeName;
use std::collections::BTreeSet;

pub(super) struct RustHeldFields;

impl RustHeldFields {
    const LOCKS: [(&'static str, &'static str, &'static str); 4] = [
        ("std::sync::Mutex<", "lock", "lock"),
        ("Mutex<", "lock", "lock"),
        ("std::sync::RwLock<", "read", "write"),
        ("RwLock<", "read", "write"),
    ];
    const UNLOCKED: &'static str = ".unwrap_or_else(|error| error.into_inner())";

    pub(super) fn table(
        source: &str,
        path: &str,
        saved: &BTreeSet<String>,
    ) -> Result<String, Failure> {
        let (name, body) = Self::service(source, path)?;
        let entries = Self::entries(path, body, saved)?;
        Ok(format!(
            "pub(crate) fn tac_held_{name}(scope: &NativeServiceScope) -> ::core::result::Result<Vec<(&'static str, TacMember)>, String> {{\nlet instance = scope.held(&tac_definition_{name}())?;\nOk(vec![\n{entries}])\n}}\n"
        ))
    }

    fn service<'a>(source: &'a str, path: &str) -> Result<(String, &'a str), Failure> {
        let wanted = declared_service(path, source);
        block_named(source, "struct", wanted.as_deref())
            .ok_or_else(|| unnamed_service(path, wanted.as_deref(), "struct"))
    }

    fn entries(path: &str, body: &str, saved: &BTreeSet<String>) -> Result<String, Failure> {
        Self::saved_declarations(body, saved)
            .iter()
            .map(|(field, declared)| Self::entry(path, field, declared))
            .collect()
    }

    fn saved_declarations(body: &str, saved: &BTreeSet<String>) -> Vec<(String, String)> {
        body.lines()
            .flat_map(rust_field_declarations)
            .filter_map(|line| Self::declaration(&line))
            .filter(|(field, _)| saved.contains(&camel_case(field)))
            .collect()
    }

    fn declaration(line: &str) -> Option<(String, String)> {
        let line = line
            .trim()
            .trim_start_matches("pub(crate) ")
            .trim_start_matches("pub ");
        let (field, declared) = line.split_once(':')?;
        let field = field.trim();
        let declared = declared.trim().trim_end_matches(',').trim();
        Self::is_name(field).then(|| (field.to_owned(), declared.to_owned()))
    }

    fn is_name(field: &str) -> bool {
        [!field.is_empty(), UnicodeName::leading(field) == field] == [true; 2]
    }

    fn entry(path: &str, field: &str, declared: &str) -> Result<String, Failure> {
        let (inner, read, write) =
            Self::lock(declared).ok_or_else(|| Self::unlocked(path, field, declared))?;
        let kind =
            Self::value_type(field, inner).ok_or_else(|| Self::unlocked(path, field, declared))?;
        let conversion = rust_write(&kind).ok_or_else(|| Self::unlocked(path, field, declared))?;
        let unlocked = Self::UNLOCKED;
        let value = rust_read(&kind, &format!("(*it.{field}.{read}(){unlocked}).clone()"));
        Ok(format!(
            "(\"{}\", TacMember::Field {{read: {{let it = instance.clone(); Box::new(move || {value})}}, write: Some({{let it = instance.clone(); Box::new(move |value| {{ *it.{field}.{write}(){unlocked} = {conversion}; }})}})}}),\n",
            camel_case(field)
        ))
    }

    fn lock(declared: &str) -> Option<(&str, &'static str, &'static str)> {
        Self::LOCKS.iter().find_map(|(prefix, read, write)| {
            let inner = declared.strip_prefix(prefix)?.strip_suffix('>')?;
            Some((inner.trim(), *read, *write))
        })
    }

    fn value_type(field: &str, inner: &str) -> Option<String> {
        rust_field(&format!("{field}: {inner}"))?.declared_type
    }

    fn unlocked(path: &str, field: &str, declared: &str) -> Failure {
        Failure::one(diagnostic(
            1406,
            format!(
                "Saved field '{field}: {declared}' is on a Rust service that another Rust service holds through Arc, so the page cannot restore it."
            ),
            Some(format!(
                "Arc gives shared access only. Declare the field as std::sync::Mutex<T> or std::sync::RwLock<T>, where T is a type a page field can have (a number, bool, String or model), for example '{field}: std::sync::Mutex<i64>'."
            )),
            source_span(path, 0, path.len()),
        ))
    }
}
