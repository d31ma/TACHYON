use crate::Failure;
use crate::template::TemplateFrontend;
use std::collections::BTreeSet;
use tachyon_contracts::{HandlerResponse, NativeTarget};

pub fn compile_template(source: &str, source_path: &str) -> Result<(), Failure> {
    TemplateFrontend::compile(source, source_path, &BTreeSet::new()).map(|_| ())
}

pub fn decode_response_frame(bytes: &[u8], request_id: &str) -> Result<HandlerResponse, Failure> {
    crate::handler::frame::HandlerFrame::response(bytes, request_id)
}

pub fn plan_native(target: NativeTarget, html: &str) -> Result<(), Failure> {
    crate::native::plan_for_fuzzing(target, html)
}

pub fn compile_yon_annotations(source: &str, extension: &str) -> Result<(), Failure> {
    let path = format!("server/controllers/FuzzController.{extension}");
    let path = std::path::Path::new(&path);
    crate::behavior::gateway_declarations(path, source)?;
    crate::yon::lower_with_routes(path, source, None).map(|_| ())
}

pub fn compile_tac_annotations(source: &str, extension: &str) -> Result<(), Failure> {
    let path = format!("client/pages/fuzz/tac.{extension}");
    let events = crate::events::extract(std::path::Path::new(&path), source, "tac")?;
    let source = events
        .as_ref()
        .map_or(source, |events| events.source.as_str());
    crate::behavior::lower(std::path::Path::new(&path), source, "tac").map(|_| ())
}

pub fn application_configuration(bytes: &[u8]) {
    crate::application::fuzz(bytes);
}

pub fn managed_resource_headers(bytes: &[u8]) {
    crate::handler::fuzz_resource_headers(bytes);
}
