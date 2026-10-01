use super::java_member::JavaMember;
use super::java_member_kind::JavaMemberKind;
use super::java_value_type::JavaValueType;

use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(super) struct JavaTypeMembers;

impl JavaTypeMembers {
    pub(super) fn lines(members: &[JavaMember], values: &JavaValueType) -> String {
        Self::member_lines(members, values)
    }

    pub(super) fn constructor(
        members: &[JavaMember],
        required: &[String],
        values: &JavaValueType,
    ) -> String {
        let properties = Self::properties(members, required, values);
        let optional = Self::optional(required.is_empty());
        format!("properties{optional}: {{ {properties} }}")
    }

    pub(super) fn require_properties(
        path: &Path,
        members: &[JavaMember],
        required: &[String],
    ) -> Result<(), Failure> {
        Self::check_properties(path, members, required)
    }

    fn check_properties(
        path: &Path,
        members: &[JavaMember],
        required: &[String],
    ) -> Result<(), Failure> {
        let missing = required
            .iter()
            .find(|name| !Self::has_property(members, name));
        missing.map_or(Ok(()), |name| {
            Err(Failure::one(diagnostic(
                1405,
                format!(
                    "The Java browser component property '{name}' must be a public writable field."
                ),
                None,
                source_span(&path.to_string_lossy(), 0, 1),
            )))
        })
    }

    fn has_property(members: &[JavaMember], name: &str) -> bool {
        members
            .iter()
            .filter(|member| member.name == name)
            .any(Self::writable)
    }

    fn member_lines(members: &[JavaMember], values: &JavaValueType) -> String {
        members
            .iter()
            .map(|member| Self::member_line(member, values))
            .collect()
    }

    fn member_line(member: &JavaMember, values: &JavaValueType) -> String {
        match member.kind {
            JavaMemberKind::Field { read_only } => Self::field(member, values, read_only),
            JavaMemberKind::Method => Self::method(member, values),
            _ => String::new(),
        }
    }

    fn field(member: &JavaMember, values: &JavaValueType, read_only: bool) -> String {
        let name = Self::quoted(&member.name);
        let kind = Self::value_type(values, &member.value_type);
        let modifier = Self::modifier(read_only);
        format!("    {modifier}{name}: {kind};\n")
    }

    fn method(member: &JavaMember, values: &JavaValueType) -> String {
        let name = Self::quoted(&member.name);
        let parameters = Self::parameters(&member.parameters, values);
        let kind = Self::value_type(values, &member.value_type);
        format!("    {name}({parameters}): {kind};\n")
    }

    fn parameters(parameters: &[String], values: &JavaValueType) -> String {
        parameters
            .iter()
            .enumerate()
            .map(|(index, kind)| format!("arg{index}: {}", Self::value_type(values, kind)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn value_type(values: &JavaValueType, kind: &str) -> String {
        values
            .typescript(kind)
            .unwrap_or_else(|| String::from("never"))
    }

    fn properties(members: &[JavaMember], required: &[String], values: &JavaValueType) -> String {
        members
            .iter()
            .filter(|member| Self::writable(member))
            .map(|member| Self::property(member, required, values))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn writable(member: &JavaMember) -> bool {
        matches!(member.kind, JavaMemberKind::Field { read_only: false })
    }

    fn modifier(read_only: bool) -> &'static str {
        if read_only { "readonly " } else { "" }
    }

    fn property(member: &JavaMember, required: &[String], values: &JavaValueType) -> String {
        let optional = Self::optional(!required.contains(&member.name));
        let name = Self::quoted(&member.name);
        format!(
            "{name}{optional}: {};",
            Self::value_type(values, &member.value_type)
        )
    }

    fn optional(optional: bool) -> &'static str {
        if optional { "?" } else { "" }
    }

    fn quoted(name: &str) -> String {
        serde_json::to_string(name).unwrap_or_default()
    }
}
