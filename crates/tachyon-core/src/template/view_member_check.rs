use super::{ServiceDefinition, ServiceRegistry};
use crate::Failure;
use crate::companion::go::GoTable;
use crate::failure::{diagnostic, source_span};
use crate::tac_web::WebLanguage;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use tachyon_diagnostics::Diagnostic;

pub(crate) struct ViewMemberCheck;

impl ViewMemberCheck {
    const CODE: u16 = 1311;

    pub(crate) fn run(
        services: &ServiceRegistry,
        nodes: &[Value],
        source_path: &str,
    ) -> Result<(), Failure> {
        let reads: BTreeSet<(String, String)> = nodes.iter().flat_map(Self::reads_in).collect();
        let instances: BTreeSet<&str> = reads
            .iter()
            .map(|(instance, _)| instance.as_str())
            .collect();
        let members: BTreeMap<&str, (&str, Vec<String>)> = instances
            .into_iter()
            .filter_map(|instance| Some((instance, Self::go_members(services.get(instance)?)?)))
            .collect();
        let found: Vec<Diagnostic> = reads
            .iter()
            .filter_map(|read| Self::near_miss(&members, read, source_path))
            .chain(super::rust_private_read::RustPrivateRead::found(
                services,
                &reads,
                source_path,
            ))
            .collect();
        (!found.is_empty())
            .then(|| Failure::new(found))
            .map_or(Ok(()), Err)
    }

    fn reads_in(value: &Value) -> Vec<(String, String)> {
        let nested = Self::nested_template(value);
        let inside = Self::children(value)
            .filter(|child| !nested.is_some_and(|template| std::ptr::eq(template, *child)))
            .flat_map(Self::reads_in);
        Self::member_read(value)
            .into_iter()
            .chain(Self::path_read(value.get("handler")))
            .chain(Self::path_read(value.pointer("/assign/target")))
            .chain(inside)
            .collect()
    }

    fn children(value: &Value) -> impl Iterator<Item = &Value> {
        let members = value
            .as_object()
            .into_iter()
            .flat_map(serde_json::Map::values);
        members.chain(value.as_array().into_iter().flatten())
    }

    fn nested_template(value: &Value) -> Option<&Value> {
        let component = value.get("k").and_then(Value::as_str) == Some("component");
        value.get("template").filter(|_| component)
    }

    fn member_read(value: &Value) -> Option<(String, String)> {
        let reads = value.get("k").and_then(Value::as_str) == Some("get");
        let object = value.get("o")?;
        let named = object.get("k").and_then(Value::as_str) == Some("id");
        let instance = object.get("n").and_then(Value::as_str)?;
        let member = value.get("p").and_then(Value::as_str)?;
        let read = (instance.to_owned(), member.to_owned());
        ([reads, named] == [true, true]).then_some(read)
    }

    fn path_read(path: Option<&Value>) -> Option<(String, String)> {
        let mut parts = path.and_then(Value::as_str)?.split('.');
        let instance = parts.next()?;
        let member = parts.next()?;
        Some((instance.to_owned(), member.to_owned()))
    }

    fn go_members(service: &ServiceDefinition) -> Option<(&str, Vec<String>)> {
        let (language, path) = service.web_assembly()?;
        let source = (language == WebLanguage::Go)
            .then_some(path)
            .and_then(|path| std::fs::read_to_string(path).ok())?;
        let keys = GoTable {
            source: &source,
            source_path: &path.to_string_lossy(),
            class: service.class(),
            excluded: &[],
            extra: "",
            needs: "",
        }
        .member_keys()?;
        Some((service.class(), keys))
    }

    fn near_miss(
        members: &BTreeMap<&str, (&str, Vec<String>)>,
        (instance, member): &(String, String),
        source_path: &str,
    ) -> Option<Diagnostic> {
        let (class, exposed) = members.get(instance.as_str())?;
        let known = exposed.iter().any(|name| name == member);
        let suggestion = exposed
            .iter()
            .find(|name| name.eq_ignore_ascii_case(member))
            .filter(|_| !known)?;
        let message = format!(
            "The view reads {instance}.{member}, but {member} is not a member of {class}; did you mean {suggestion}?"
        );
        let help = format!(
            "A Go service reaches the view with only the first letter of each exported name lowered: {} is {suggestion}. Write {instance}.{suggestion}.",
            crate::go_source::upper_first(suggestion)
        );
        Some(diagnostic(
            Self::CODE,
            message,
            Some(help),
            source_span(source_path, 0, 0),
        ))
    }
}
