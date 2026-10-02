use super::{
    CompanionLanguage, KotlinMember, Member, class_body_named, closing_parenthesis,
    declared_service, members, parameter_type, unnamed_service,
};
use crate::Failure;
use crate::events::EventModule;
use crate::failure::{diagnostic, source_span};
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) struct KotlinMembers;

impl KotlinMembers {
    pub(crate) fn public_declaration(mut source: &str) -> Option<&str> {
        loop {
            match source.split_whitespace().next()? {
                "private" | "protected" | "internal" => return None,
                modifier @ ("public" | "lateinit") => {
                    source = source[modifier.len()..].trim_start();
                }
                _ => return Some(source),
            }
        }
    }

    pub(crate) fn read(path: &Path, source: &str) -> Result<Vec<KotlinMember>, Failure> {
        let events = crate::events::extract(path, source, "tac")?;
        let excluded = Self::excluded(path, source, events.as_ref());
        let source = events
            .as_ref()
            .map_or(source, |module| module.source.as_str());
        let masked = crate::annotation_layout::masked_source(path, source)?;
        Self::owned(path, &masked, &excluded)
    }

    fn excluded(path: &Path, source: &str, events: Option<&EventModule>) -> BTreeSet<String> {
        let mut names = Self::event_members(events);
        let class = declared_service(&path.to_string_lossy(), source).unwrap_or_default();
        names.extend(crate::template::RequiredFields::injected_fields(
            path,
            source,
            &class,
            &BTreeSet::new(),
        ));
        names
    }

    fn event_members(events: Option<&EventModule>) -> BTreeSet<String> {
        events
            .into_iter()
            .flat_map(|module| module.methods.iter().map(|method| method.name.clone()))
            .collect()
    }

    fn owned(
        path: &Path,
        source: &str,
        excluded: &BTreeSet<String>,
    ) -> Result<Vec<KotlinMember>, Failure> {
        let shown = path.to_string_lossy();
        let wanted = declared_service(&shown, source);
        let (_, body) = class_body_named(source, wanted.as_deref())
            .ok_or_else(|| unnamed_service(&shown, wanted.as_deref(), "class"))?;
        let found = members(CompanionLanguage::Kotlin, body, &shown)?;
        found
            .iter()
            .filter(|member| Self::visible(member, excluded))
            .map(|member| Self::typed(path, source, member))
            .collect()
    }

    fn visible(member: &Member, excluded: &BTreeSet<String>) -> bool {
        !excluded.contains(&member.name) && !Self::publisher(member)
    }

    fn publisher(member: &Member) -> bool {
        member
            .declared_type
            .as_deref()
            .map(str::trim)
            .map(|kind| kind.trim_end_matches('?'))
            == Some("ApplicationEventPublisher")
    }

    fn typed(path: &Path, source: &str, member: &Member) -> Result<KotlinMember, Failure> {
        let value_type = Self::declared_type(source, member)
            .ok_or_else(|| Self::unknown_type(path, source, member))?;
        Self::require_type(path, source, member, &value_type)?;
        let parameters = member
            .parameters
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|parameter| String::from(parameter_type(parameter)))
            .collect();
        Ok(KotlinMember {
            name: member.name.clone(),
            value_type,
            parameters,
            method: member.method,
            mutable: member.mutable,
        })
    }

    fn declared_type(source: &str, member: &Member) -> Option<String> {
        member
            .declared_type
            .clone()
            .or_else(|| Self::block_unit(source, member))
    }

    fn block_unit(source: &str, member: &Member) -> Option<String> {
        Self::method_signature(source, member).and_then(Self::signature_unit)
    }

    fn method_signature<'a>(source: &'a str, member: &Member) -> Option<&'a str> {
        member
            .method
            .then(|| source.get(Self::member_at(source, member) + member.name.len()..))
            .flatten()
    }

    fn signature_unit(signature: &str) -> Option<String> {
        let close = closing_parenthesis(signature)?;
        signature[close + 1..]
            .trim_start()
            .starts_with('{')
            .then(|| String::from("Unit"))
    }

    fn require_type(path: &Path, source: &str, member: &Member, kind: &str) -> Result<(), Failure> {
        Self::shape(kind).map_err(|message| Self::failure(path, source, member, &message))
    }

    fn shape(kind: &str) -> Result<(), String> {
        if kind == "Unit" {
            return Ok(());
        }
        crate::route_types::TypeText::shape("kt", kind).map(|_| ())
    }

    fn unknown_type(path: &Path, source: &str, member: &Member) -> Failure {
        Self::failure(
            path,
            source,
            member,
            "Kotlin browser facade metadata requires an explicit public member type; inferred returns and values remain supported by the existing runtime, but cannot yet provide composed types.",
        )
    }

    fn failure(path: &Path, source: &str, member: &Member, message: &str) -> Failure {
        let at = Self::member_at(source, member);
        Failure::one(diagnostic(
            1405,
            message,
            Some(String::from(
                "Declare the Kotlin member's return or property type explicitly before using composed type generation.",
            )),
            source_span(&path.to_string_lossy(), at, at + member.name.len()),
        ))
    }

    fn member_at(source: &str, member: &Member) -> usize {
        let prefix = if member.method { "fun " } else { "" };
        source
            .find(&format!("{prefix}{}", member.name))
            .map_or(0, |at| at + prefix.len())
    }
}
