use super::kotlin_type_values::KotlinTypeValues;
use crate::Failure;
use crate::companion::KotlinMember;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(super) struct KotlinTypeMembers;

impl KotlinTypeMembers {
    pub(super) fn lines(
        path: &Path,
        members: &[KotlinMember],
        values: &KotlinTypeValues,
    ) -> Result<String, Failure> {
        members
            .iter()
            .map(|member| Self::member_line(path, member, values))
            .collect()
    }

    pub(super) fn constructor(
        path: &Path,
        members: &[KotlinMember],
        required: &[String],
        values: &KotlinTypeValues,
    ) -> Result<String, Failure> {
        Self::require_properties(path, members, required)?;
        Self::properties(path, members, required, values)
    }

    fn member_line(
        path: &Path,
        member: &KotlinMember,
        values: &KotlinTypeValues,
    ) -> Result<String, Failure> {
        let render = Self::renderer(member.method);
        render(path, member, values)
    }

    fn renderer(
        method: bool,
    ) -> fn(&Path, &KotlinMember, &KotlinTypeValues) -> Result<String, Failure> {
        if method { Self::method } else { Self::field }
    }

    fn method(
        path: &Path,
        member: &KotlinMember,
        values: &KotlinTypeValues,
    ) -> Result<String, Failure> {
        let parameters = Self::parameters(path, &member.parameters, values)?;
        let kind = Self::value(path, &member.value_type, values)?;
        Ok(format!(
            "    {}({parameters}): {kind};\n",
            Self::quoted(&member.name)
        ))
    }

    fn field(
        path: &Path,
        member: &KotlinMember,
        values: &KotlinTypeValues,
    ) -> Result<String, Failure> {
        let kind = Self::data_value(path, &member.value_type, values)?;
        let modifier = if member.mutable { "" } else { "readonly " };
        Ok(format!(
            "    {modifier}{}: {kind};\n",
            Self::quoted(&member.name)
        ))
    }

    fn parameters(
        path: &Path,
        parameters: &[String],
        values: &KotlinTypeValues,
    ) -> Result<String, Failure> {
        parameters
            .iter()
            .enumerate()
            .map(|(index, kind)| {
                Self::data_value(path, kind, values).map(|kind| format!("arg{index}: {kind}"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|parameters| parameters.join(", "))
    }

    fn value(path: &Path, kind: &str, values: &KotlinTypeValues) -> Result<String, Failure> {
        values
            .typescript(kind)
            .map_err(|message| Self::failure(path, &message))
    }

    fn data_value(path: &Path, kind: &str, values: &KotlinTypeValues) -> Result<String, Failure> {
        values
            .shape(kind)
            .map(|shape| KotlinTypeValues::rendered(&shape))
            .map_err(|message| Self::failure(path, &message))
    }

    fn require_properties(
        path: &Path,
        members: &[KotlinMember],
        required: &[String],
    ) -> Result<(), Failure> {
        let missing = required
            .iter()
            .find(|name| !Self::has_property(members, name));
        missing.map_or(Ok(()), |name| {
            Err(Self::failure(
                path,
                &format!(
                    "Kotlin component property '{name}' needs a public explicitly typed field."
                ),
            ))
        })
    }

    fn has_property(members: &[KotlinMember], name: &str) -> bool {
        members
            .iter()
            .any(|member| Self::named_property(member, name))
    }

    fn named_property(member: &KotlinMember, name: &str) -> bool {
        !member.method && member.name == name
    }

    fn constructor_property(member: &KotlinMember, required: &[String]) -> bool {
        match (member.method, member.mutable) {
            (true, _) => false,
            (false, true) => true,
            (false, false) => required.contains(&member.name),
        }
    }

    fn optional(required: &[String]) -> &'static str {
        if required.is_empty() { "?" } else { "" }
    }

    fn properties(
        path: &Path,
        members: &[KotlinMember],
        required: &[String],
        values: &KotlinTypeValues,
    ) -> Result<String, Failure> {
        let properties = members
            .iter()
            .filter(|member| Self::constructor_property(member, required))
            .map(|member| Self::property(path, member, required, values))
            .collect::<Result<Vec<_>, _>>()?
            .join(" ");
        let optional = Self::optional(required);
        Ok(format!("properties{optional}: {{ {properties} }}"))
    }

    fn property(
        path: &Path,
        member: &KotlinMember,
        required: &[String],
        values: &KotlinTypeValues,
    ) -> Result<String, Failure> {
        let optional = if required.contains(&member.name) {
            ""
        } else {
            "?"
        };
        let kind = Self::data_value(path, &member.value_type, values)?;
        Ok(format!("{}{optional}: {kind};", Self::quoted(&member.name)))
    }

    fn quoted(name: &str) -> String {
        serde_json::to_string(name).unwrap_or_default()
    }

    fn failure(path: &Path, message: &str) -> Failure {
        Failure::one(diagnostic(
            1405,
            message,
            None,
            source_span(&path.to_string_lossy(), 0, 1),
        ))
    }
}
