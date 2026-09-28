use super::super::gateway::TargetSignature;
use super::{Field, Input, Method, Parameter, Type, brace_depth, is_reserved_type_name};
use crate::Failure;
use crate::go_source::{self, GoType, GoTypeKind};
use std::collections::BTreeSet;
use std::ops::Range;

pub(super) fn declarations(input: &Input<'_>, skip_runtime: bool) -> Result<Vec<Type>, Failure> {
    let mut result = Vec::new();
    let mut names = BTreeSet::new();
    for declared in go_source::types(&input.code) {
        let name = declared.name.as_str();
        if skip_runtime && is_reserved_type_name(name) {
            continue;
        }
        if !super::super::identifier(name) || is_reserved_type_name(name) {
            return Err(input.failure(declared.start, "Behavior type requires a safe explicit name outside the reserved Behavior API namespace"));
        }
        if !names.insert(name.to_owned()) {
            return Err(input.failure(declared.start, "Duplicate top-level behavior type name"));
        }
        result.push(behavior_type(input, &declared));
        if result.len() > 128 {
            return Err(input.failure(declared.start, "Behavior module exceeds 128 types"));
        }
    }
    Ok(result)
}

fn behavior_type(input: &Input<'_>, declared: &GoType) -> Type {
    let fields = if declared.kind == GoTypeKind::Struct {
        struct_fields(input, declared.body.clone())
    } else {
        Vec::new()
    };
    Type {
        name: declared.name.clone(),
        declaration: declared.start,
        keyword: declared.keyword.clone(),
        declared: declared.kind == GoTypeKind::Interface,
        body: declared.body.clone(),
        fields,
        methods: Vec::new(),
        configuration: false,
        cache: None,
        entity: None,
        table: None,
        constructor: None,
        repository: None,
        policies: serde_json::Map::new(),
    }
}

fn struct_fields(input: &Input<'_>, body: Range<usize>) -> Vec<Field> {
    let mut fields = Vec::new();
    for field in go_source::fields(&input.code, body) {
        if !super::super::identifier(&field.name) {
            continue;
        }
        fields.push(Field {
            name: field.name,
            type_source: field.type_source,
            uninitialized: true,
            initializer: None,
            is_static: false,
        });
    }
    fields
}

pub(super) fn method_owner(input: &Input<'_>, types: &[Type], at: usize) -> Option<usize> {
    if let Some(method) = go_source::method_at(&input.code, at) {
        return types.iter().position(|ty| ty.name == method.receiver_type);
    }
    types.iter().position(|ty| {
        ty.declared
            && at > ty.body.start
            && at < ty.body.end
            && brace_depth(&input.code[ty.body.start..at]) == 0
    })
}

pub(super) fn parse_method(input: &Input<'_>, at: usize, ty: &Type) -> Result<Method, Failure> {
    if let Some(method) = go_source::method_at(&input.code, at) {
        let parameters = parameters(input, at, &input.code[method.parameters.clone()])?;
        return Ok(Method {
            name: method.name,
            parameters,
            signature: at..method.body.start.saturating_sub(1),
            body: method.body,
            return_type: input.code[method.results].trim().to_owned(),
            asynchronous: false,
            is_static: false,
            declared: false,
            metadata: serde_json::Map::new(),
        });
    }
    if ty.declared {
        let specs = go_source::interface_methods(&input.code, ty.body.clone());
        if let Some(spec) = specs.into_iter().find(|spec| spec.start == at) {
            let parameters = parameters(input, at, &input.code[spec.parameters.clone()])?;
            let end = spec.results.end;
            return Ok(Method {
                name: spec.name,
                parameters,
                signature: at..end,
                body: end..end,
                return_type: input.code[spec.results].trim().to_owned(),
                asynchronous: false,
                is_static: false,
                declared: true,
                metadata: serde_json::Map::new(),
            });
        }
    }
    Err(input.failure(at, "Behavior methods must be named concrete declarations"))
}

fn parameters(input: &Input<'_>, at: usize, text: &str) -> Result<Vec<Parameter>, Failure> {
    let mut parameters = Vec::new();
    for parameter in go_source::parameters(text) {
        if parameter.name.is_empty() {
            return Err(input.failure(
                at,
                "Behavior parameters require explicit names and native types",
            ));
        }
        if parameter.type_source.starts_with("...") {
            return Err(input.failure(at, "Behavior methods require explicit positional parameters without defaults or rest arguments"));
        }
        if parameter.name != "_" && !super::super::identifier(&parameter.name) {
            return Err(input.failure(at, "Invalid behavior parameter name"));
        }
        if parameter.name.starts_with("__tachyon") {
            return Err(input.failure(at, "Behavior parameter uses a compiler-reserved name"));
        }
        parameters.push(Parameter {
            name: parameter.name,
            type_source: parameter.type_source,
        });
    }
    if parameters.len() > 32 {
        return Err(input.failure(at, "Behavior methods exceed 32 parameters"));
    }
    Ok(parameters)
}

pub(super) fn target_signatures(
    input: &Input<'_>,
    ty: &Type,
    requested_method: &str,
) -> Vec<TargetSignature> {
    let exported_name = go_source::upper_first(requested_method);
    let mut signatures = Vec::new();
    for method in go_source::methods(&input.code) {
        if method.receiver_type != ty.name || method.name != exported_name {
            continue;
        }
        let mut parameter_types = Vec::new();
        for parameter in go_source::parameters(&input.code[method.parameters]) {
            parameter_types.push(parameter.type_source);
        }
        signatures.push(TargetSignature {
            parameter_types,
            result_type: input.code[method.results].trim().to_owned(),
            is_static: false,
            has_body: true,
            has_supported_receiver: true,
        });
    }
    signatures
}

pub(super) fn test_cases(input: &Input<'_>, ty: &Type) -> Vec<String> {
    let mut names = Vec::new();
    for method in go_source::methods(&input.code) {
        if method.receiver_type != ty.name {
            continue;
        }
        let is_case = method.name.starts_with("test") || method.name.starts_with("Test");
        if is_case && super::super::identifier(&method.name) && !names.contains(&method.name) {
            names.push(method.name);
        }
    }
    names
}
