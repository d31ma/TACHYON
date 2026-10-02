use super::super::{Input, Marker};
use super::{Field, Type};
use crate::Failure;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct CsharpTacHelpers;

impl CsharpTacHelpers {
    const MANAGED: [&'static str; 3] = ["Needs", "NeedsAll", "Lifetime"];
    const HOOKS: [&'static str; 3] = ["OnStart", "OnStop", "OnEvent"];

    pub(super) fn source(path: &Path, source: &str) -> Result<(), Failure> {
        Self::source_if(path, source)
    }

    pub(super) fn retains_marker(path: &Path) -> bool {
        Self::client_path(path)
    }

    pub(super) fn captured(
        input: &Input<'_>,
        language: &str,
        environment: &str,
        markers: &[Marker],
    ) -> Result<(), Failure> {
        Self::wanted(input.path, language, environment)
            .then(|| Self::captured_types(input, markers))
            .transpose()
            .map(|_| ())
    }

    pub(super) fn constructed(
        input: &Input<'_>,
        language: &str,
        environment: &str,
        types: &[Type],
    ) -> Result<(), Failure> {
        Self::wanted(input.path, language, environment)
            .then(|| Self::constructed_types(input, types))
            .transpose()
            .map(|_| ())
    }

    fn wanted(path: &Path, language: &str, environment: &str) -> bool {
        [
            language == "cs",
            environment == "tac",
            Self::client_path(path),
        ] == [true, true, true]
    }

    fn client_path(path: &Path) -> bool {
        let extension = path.extension().and_then(|value| value.to_str());
        let realm = crate::annotation_rules::Realm::of(path);
        (extension, realm) == (Some("cs"), Some(crate::annotation_rules::Realm::Tac))
    }

    fn source_if(path: &Path, source: &str) -> Result<(), Failure> {
        if !Self::client_path(path) {
            return Ok(());
        }
        let mask = crate::lexical::code_mask("cs", source);
        let input = Input {
            path,
            source,
            code: super::visible_code(source, mask.code),
        };
        super::check_behavior_source(&input, mask.error, "tac")?;
        let markers = super::annotations(&input, "cs", "tac")?;
        Self::captured_types(&input, &markers)
    }

    fn captured_types(input: &Input<'_>, markers: &[Marker]) -> Result<(), Failure> {
        let targets = Self::helper_targets(markers);
        Self::captured_targets(input, markers, &targets)
    }

    fn captured_targets(
        input: &Input<'_>,
        markers: &[Marker],
        targets: &BTreeSet<usize>,
    ) -> Result<(), Failure> {
        if targets.is_empty() {
            return Ok(());
        }
        Self::captured_declarations(input, markers, targets)
    }

    fn captured_declarations(
        input: &Input<'_>,
        markers: &[Marker],
        targets: &BTreeSet<usize>,
    ) -> Result<(), Failure> {
        let mut code = Input {
            path: input.path,
            source: input.source,
            code: input.code.clone(),
        };
        super::mask_annotations(&mut code, markers, "cs");
        let types = super::declarations(&code, "cs")?;
        let helpers = Self::captured_helpers(&types, targets);
        Self::captured_roles(input, &helpers, markers)?;
        let names = helpers.iter().map(|ty| ty.name.as_str()).collect();
        Self::captured_dependencies(input, &types, markers, &names)
    }

    fn captured_helpers<'a>(types: &'a [Type], targets: &BTreeSet<usize>) -> Vec<&'a Type> {
        types
            .iter()
            .filter(|ty| targets.contains(&ty.declaration))
            .collect()
    }

    fn captured_roles(
        input: &Input<'_>,
        helpers: &[&Type],
        markers: &[Marker],
    ) -> Result<(), Failure> {
        helpers
            .iter()
            .try_for_each(|ty| Self::captured_type(input, ty, markers))
    }

    fn captured_dependencies(
        input: &Input<'_>,
        types: &[Type],
        markers: &[Marker],
        helpers: &BTreeSet<&str>,
    ) -> Result<(), Failure> {
        markers
            .iter()
            .try_for_each(|marker| Self::captured_dependency(input, types, marker, helpers))
    }

    fn captured_dependency(
        input: &Input<'_>,
        types: &[Type],
        marker: &Marker,
        helpers: &BTreeSet<&str>,
    ) -> Result<(), Failure> {
        let fields = Self::requested_fields(input, types, marker)?;
        fields.iter().try_for_each(|field| {
            Self::require_dependency(input, marker.span.start, &field.type_source, helpers)
        })
    }

    fn requested_fields<'a>(
        input: &Input<'_>,
        types: &'a [Type],
        marker: &Marker,
    ) -> Result<Vec<&'a Field>, Failure> {
        match marker.name.as_str() {
            "NeedsAll" => Ok(Self::class_fields(types, marker.target)),
            "Needs" => Self::field_fields(input, types, marker),
            _ => Ok(Vec::new()),
        }
    }

    fn class_fields(types: &[Type], target: usize) -> Vec<&Field> {
        types
            .iter()
            .find(|ty| ty.declaration == target)
            .map_or_else(Vec::new, Self::needed_fields)
    }

    fn needed_fields(ty: &Type) -> Vec<&Field> {
        ty.fields
            .iter()
            .filter(|field| (field.uninitialized, field.is_static) == (true, false))
            .collect()
    }

    fn field_fields<'a>(
        input: &Input<'_>,
        types: &'a [Type],
        marker: &Marker,
    ) -> Result<Vec<&'a Field>, Failure> {
        let owner = super::field_owner(input, "cs", types, marker.target)?;
        let name = super::super::validation::field_name(input, marker, "cs")?;
        Ok(types[owner]
            .fields
            .iter()
            .filter(|field| field.name == name)
            .collect())
    }

    fn helper_targets(markers: &[Marker]) -> BTreeSet<usize> {
        markers
            .iter()
            .filter(|marker| marker.name == "Helper")
            .map(|marker| marker.target)
            .collect()
    }

    fn captured_type(input: &Input<'_>, ty: &Type, markers: &[Marker]) -> Result<(), Failure> {
        Self::captured_hooks(input, ty)?;
        markers
            .iter()
            .filter(|marker| Self::unsupported_marker(ty, marker))
            .try_for_each(|marker| Err(Self::failure(input, marker.span.start, &ty.name)))
    }

    fn captured_hooks(input: &Input<'_>, ty: &Type) -> Result<(), Failure> {
        let mut offset = ty.body.start;
        for line in input.code[ty.body.clone()].split_inclusive('\n') {
            Self::require_hook(input, ty, line, offset)?;
            offset += line.len();
        }
        Ok(())
    }

    fn require_hook(
        input: &Input<'_>,
        ty: &Type,
        line: &str,
        offset: usize,
    ) -> Result<(), Failure> {
        if Self::hook_name(line).is_some() {
            return Err(Self::failure(
                input,
                offset + line.len() - line.trim_start().len(),
                &ty.name,
            ));
        }
        Ok(())
    }

    fn hook_name(line: &str) -> Option<&str> {
        let written = line.trim_start().strip_prefix('[')?;
        let qualified = written.split(['(', ']', ' ']).next()?;
        let name = qualified.rsplit('.').next()?;
        Self::HOOKS.contains(&name).then_some(name)
    }

    fn unsupported_marker(ty: &Type, marker: &Marker) -> bool {
        let class_owned = (
            marker.target == ty.declaration,
            Self::MANAGED.contains(&marker.name.as_str()),
        );
        let hook_owned = (
            ty.body.contains(&marker.target),
            Self::HOOKS.contains(&marker.name.as_str()),
        );
        [class_owned, hook_owned].contains(&(true, true))
    }

    fn constructed_types(input: &Input<'_>, types: &[Type]) -> Result<(), Failure> {
        let helpers = Self::helpers(types);
        let names = helpers.iter().map(|ty| ty.name.as_str()).collect();
        helpers
            .iter()
            .try_for_each(|ty| Self::require_caller(input, ty))?;
        types
            .iter()
            .try_for_each(|ty| Self::require_manual_dependencies(input, ty, &names))
    }

    fn helpers(types: &[Type]) -> Vec<&Type> {
        types
            .iter()
            .filter(|ty| ty.policies.contains_key("helper"))
            .collect()
    }

    fn require_caller(input: &Input<'_>, ty: &Type) -> Result<(), Failure> {
        let mut outside = input.source.to_owned();
        outside.replace_range(
            ty.body.clone(),
            &super::blank_out(&input.source[ty.body.clone()]),
        );
        let selected = crate::yon::ManualHelpers::read(input.path, &outside)?.contains(&ty.name);
        let mut code = input.code.clone();
        code.replace_range(ty.body.clone(), &super::blank_out(&code[ty.body.clone()]));
        let called = [selected, Self::external_new(&code, &ty.name)] == [true, true];
        called
            .then_some(())
            .ok_or_else(|| Self::failure(input, ty.declaration, &ty.name))
    }

    fn external_new(code: &str, name: &str) -> bool {
        code.match_indices(name)
            .any(|(at, _)| Self::new_call(code, name, at))
    }

    fn new_call(code: &str, name: &str, at: usize) -> bool {
        let before = code[..at].split_whitespace().next_back();
        let after = code[at + name.len()..].trim_start();
        [before == Some("new"), after.starts_with('(')] == [true, true]
    }

    fn require_manual_dependencies(
        input: &Input<'_>,
        ty: &Type,
        helpers: &BTreeSet<&str>,
    ) -> Result<(), Failure> {
        ty.constructor
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter_map(|name| ty.fields.iter().find(|field| field.name == *name))
            .try_for_each(|field| {
                Self::require_dependency(input, ty.declaration, &field.type_source, helpers)
            })
    }

    fn require_dependency(
        input: &Input<'_>,
        at: usize,
        kind: &str,
        helpers: &BTreeSet<&str>,
    ) -> Result<(), Failure> {
        let helper = kind
            .split(Self::type_separator)
            .find(|name| helpers.contains(name));
        helper.map_or(Ok(()), |name| Err(Self::failure(input, at, name)))
    }

    fn type_separator(character: char) -> bool {
        ![character.is_ascii_alphanumeric(), character == '_'].contains(&true)
    }

    fn failure(input: &Input<'_>, at: usize, name: &str) -> Failure {
        input.failure(at, format!("Tac C# Helper {name} requires explicit caller-owned construction outside its own class; managed Helper injection, lifetime and lifecycle hooks are not supported. Construct it in a same-language Setup or Service and retain the caller's ownership."))
    }
}
