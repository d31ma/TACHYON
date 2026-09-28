use super::{Input, Registration};
use crate::Failure;
use crate::events::{EventMethod, EventParameter};
use std::path::Path;

pub(crate) fn event_target(
    path: &Path,
    source: &str,
    code: &str,
    at: usize,
    field: bool,
) -> Result<EventMethod, Failure> {
    let input = Input {
        path,
        source,
        code: code.to_owned(),
    };
    let language = crate::groovy_source::GroovySource::syntax(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or(""),
    );
    if language == "rb" {
        return super::ruby::RubyBehaviors::event_target(&input, at, field);
    }
    if !matches!(language, "js" | "ts") {
        return if field {
            super::polyglot::event_field(&input, language, at)
        } else {
            super::polyglot::event_target(&input, language, at)
        };
    }
    let registrations = super::classes(&input)?;
    let Some(owner) = registrations
        .iter()
        .find(|owner| at > owner.open && at < owner.close)
    else {
        return Err(input.failure(
            at,
            "Event annotations require a named class instance member",
        ));
    };
    if field {
        return field_target(&input, owner, at);
    }
    method_target(&input, owner, at)
}

fn field_target(
    input: &Input<'_>,
    owner: &Registration,
    at: usize,
) -> Result<EventMethod, Failure> {
    let code = &input.code;
    let end = code[at..].find('\n').map_or(code.len(), |end| at + end);
    let declaration = input.source[at..end].trim().trim_end_matches(';').trim();
    let text_before_type = declaration.split(':').next().unwrap_or("");
    let name = text_before_type.trim().trim_end_matches(['!', '?']);
    if !super::identifier(name) || declaration.contains(['=', '(', ')']) {
        return Err(input.failure(
            at,
            "Resource requires an uninitialized named instance field",
        ));
    }
    Ok(EventMethod {
        owner: owner.name.clone(),
        name: name.to_owned(),
        parameters: Vec::new(),
        signature: at..end,
        body: end..end,
        return_type: String::new(),
        asynchronous: false,
    })
}

fn method_target(
    input: &Input<'_>,
    owner: &Registration,
    at: usize,
) -> Result<EventMethod, Failure> {
    let method = super::method(input, owner, at)?;
    if method.is_static {
        return Err(input.failure(
            at,
            "Event and lifecycle annotations require instance methods",
        ));
    }
    let parameter_source = &input.source[method.parameters.clone()];
    let mut parameters = Vec::new();
    for parameter in super::polyglot::split_parameters(parameter_source) {
        let (name, type_source) = parameter.split_once(':').unwrap_or((parameter, ""));
        if !super::identifier(name.trim()) || parameter.contains('=') {
            return Err(input.failure(at, "Event methods require simple positional parameters"));
        }
        parameters.push(EventParameter {
            name: name.trim().to_owned(),
            type_source: type_source.trim().to_owned(),
        });
    }
    let return_type = return_type(input, method.parameters.end, method.body.start);
    Ok(EventMethod {
        owner: owner.name.clone(),
        name: method.name,
        parameters,
        signature: at..method.body.start.saturating_sub(1),
        body: method.body,
        return_type,
        asynchronous: method.asynchronous,
    })
}

fn return_type(input: &Input<'_>, parameters_end: usize, body_start: usize) -> String {
    let Some(between) = input.source.get(parameters_end + 1..body_start) else {
        return String::new();
    };
    let Some(declared) = between.trim().strip_prefix(':') else {
        return String::new();
    };
    declared.trim().trim_end_matches('{').trim().to_owned()
}
