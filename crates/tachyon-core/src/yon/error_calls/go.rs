use super::{ErrorCalls, Plan, RouteCalls};
use crate::go_source::GoFile;
use crate::yon::go_container::{GoContainer, GoRouteResult};
use crate::yon::plan::ErrorMethod;
use std::fmt::Write as _;

pub(super) fn functions(plan: &Plan, stripped: &str) -> Result<String, String> {
    let parameters = ErrorCalls::parameters(plan, "go");
    let mut source = format!("\n{}errors\npackage main\n", GoFile::MARKER);
    if plan.errors.iter().any(is_concrete) {
        source.push_str("import __yonErrors \"errors\"\n");
    }
    let _ = writeln!(
        source,
        "func __yon_handle_error__(failure error, request *yonRequest{parameters}) (any, error) {{\n\tselected := -1"
    );
    declare_error_values(&mut source, plan);
    match_outermost_error(&mut source, plan);
    match_joined_errors(&mut source, plan);
    match_any_error(&mut source, plan);
    source.push_str("\tswitch selected {\n");
    for (position, method) in plan.errors.iter().enumerate() {
        call_handler(&mut source, plan, stripped, position, method)?;
    }
    source.push_str("\t}\n\treturn nil, failure\n}\n");
    Ok(source)
}

fn is_concrete(method: &ErrorMethod) -> bool {
    method.error_type != "error"
}

fn declare_error_values(source: &mut String, plan: &Plan) {
    for (position, method) in plan.errors.iter().enumerate() {
        if is_concrete(method) {
            let _ = writeln!(
                source,
                "\tvar error{position} {}\n\tvar _ error = error{position}",
                method.error_type
            );
        }
    }
}

fn match_outermost_error(source: &mut String, plan: &Plan) {
    if !plan.errors.iter().any(is_concrete) {
        return;
    }
    source.push_str(
        "\tfor current := failure; current != nil && selected < 0; current = __yonErrors.Unwrap(current) {\n",
    );
    for (position, method) in plan.errors.iter().enumerate() {
        if is_concrete(method) {
            let _ = writeln!(
                source,
                "\t\tif value, ok := current.({}); ok {{ error{position} = value; selected = {position} }}",
                method.error_type
            );
        }
    }
    source.push_str("\t}\n");
}

fn match_joined_errors(source: &mut String, plan: &Plan) {
    if !plan.errors.iter().any(is_concrete) {
        return;
    }
    source.push_str("\tif selected < 0 {\n\t\tmatches := 0\n");
    for (position, method) in plan.errors.iter().enumerate() {
        if is_concrete(method) {
            let _ = writeln!(
                source,
                "\t\tif __yonErrors.As(failure, &error{position}) {{ selected = {position}; matches++ }}"
            );
        }
    }
    source.push_str("\t\tif matches > 1 { return nil, failure }\n\t}\n");
}

fn match_any_error(source: &mut String, plan: &Plan) {
    for (position, method) in plan.errors.iter().enumerate() {
        if !is_concrete(method) {
            let _ = writeln!(source, "\tif selected < 0 {{ selected = {position} }}");
        }
    }
}

fn call_handler(
    source: &mut String,
    plan: &Plan,
    stripped: &str,
    position: usize,
    method: &ErrorMethod,
) -> Result<(), String> {
    let Some((index, _)) = RouteCalls::error_owners(plan)
        .into_iter()
        .find(|(_, name)| *name == method.owner)
    else {
        return Ok(());
    };
    let error = if is_concrete(method) {
        format!("error{position}")
    } else {
        String::from("failure")
    };
    let request = if method.takes_request {
        ", yonNativeRequest(request)"
    } else {
        ""
    };
    let call = format!("handler{index}.{}({error}{request})", method.method);
    let _ = writeln!(source, "\tcase {position}:");
    match GoContainer::route_result(stripped, &method.owner, &method.method)? {
        GoRouteResult::Value => {
            let _ = writeln!(source, "\t\tvalue := {call}");
        }
        GoRouteResult::ValueAndFailure => {
            let _ = writeln!(
                source,
                "\t\tvalue, handlerFailure := {call}\n\t\tif handlerFailure != nil {{ return nil, handlerFailure }}"
            );
        }
        GoRouteResult::Nothing | GoRouteResult::Failure => {
            return Err(String::from(
                "A Go @HandlesError method must return a response value, optionally followed by error",
            ));
        }
    }
    let _ = writeln!(
        source,
        "\t\tresponse := yonAnswer(value)\n\t\tif response.status == 200 {{ response.Status({}) }}\n\t\treturn response, nil",
        method.status
    );
    Ok(())
}
