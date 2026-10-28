use crate::Failure;
use std::path::Path;

mod argument_range;
mod base_methods;
mod call_arguments;
mod class_close;
mod delegate_base;
mod delegate_refusal;
#[cfg(test)]
mod dispatch_tests;
mod dispatch_text;
mod dynamic_dispatch;
mod forward_text;
mod fsharp_parameters;
mod marker_end;
mod marker_strip;
mod method_header;
mod name_clash;
#[cfg(test)]
mod override_tests;
mod own_methods;
mod own_signatures;
mod parameter_split;
mod signature;
mod source_lines;
#[cfg(test)]
mod tests;
mod typed_forward;
#[cfg(test)]
mod typed_tests;

use argument_range::ArgumentRange;
use base_methods::BaseMethods;
use call_arguments::CallArguments;
use class_close::ClassClose;
use delegate_base::DelegateBase;
use delegate_refusal::DelegateRefusal;
use dispatch_text::DispatchText;
use dynamic_dispatch::{DynamicDispatch, Overload};
use forward_text::ForwardText;
use fsharp_parameters::FsharpParameters;
use marker_end::MarkerEnd;
use marker_strip::MarkerStrip;
use method_header::MethodHeader;
use name_clash::NameClash;
use own_methods::OwnMethods;
use own_signatures::OwnSignatures;
use parameter_split::ParameterSplit;
use signature::Signature;
use source_lines::SourceLines;
use typed_forward::TypedForward;

pub(crate) struct DelegateForwards<'a> {
    services: Vec<(&'a Path, &'a str)>,
}

impl<'a> DelegateForwards<'a> {
    const REALMS: [&'static str; 2] = ["client", "server"];
    const SERVICE_FOLDER: &'static str = "services";
    const DELEGATE_FOLDER: &'static str = "delegates";

    pub(crate) fn new(files: impl Iterator<Item = (&'a Path, &'a [u8])>) -> Self {
        let services = files
            .filter(|(path, _)| Self::in_folder(path, Self::SERVICE_FOLDER))
            .filter_map(|(path, bytes)| Some((path, std::str::from_utf8(bytes).ok()?)))
            .collect();
        Self { services }
    }

    pub(crate) fn staged(&self, path: &Path, bytes: &[u8]) -> Result<Option<String>, Failure> {
        let source = std::str::from_utf8(bytes)
            .ok()
            .filter(|_| Self::in_folder(path, Self::DELEGATE_FOLDER));
        source
            .map(|source| self.forwarded(path, source))
            .transpose()
            .map(Option::flatten)
    }

    fn forwarded(&self, path: &Path, source: &str) -> Result<Option<String>, Failure> {
        let services: Vec<(&Path, &str)> = self
            .services
            .iter()
            .copied()
            .filter(|(service, _)| Self::same_unit(path, service))
            .collect();
        let base = DelegateBase::read(path, source, &services)?;
        base.map_or(Ok(None), |base| Self::written(path, source, &base))
    }

    fn written(
        path: &Path,
        source: &str,
        base: &DelegateBase<'_>,
    ) -> Result<Option<String>, Failure> {
        let masked = crate::annotation_layout::masked_source(path, source).ok();
        masked.map_or(Ok(None), |masked| {
            Self::with_masked(path, (source, &masked), base)
        })
    }

    fn with_masked(
        path: &Path,
        (source, masked): (&str, &str),
        base: &DelegateBase<'_>,
    ) -> Result<Option<String>, Failure> {
        let methods = BaseMethods::listed(base);
        let names: Vec<String> = methods.iter().map(|(name, _)| name.clone()).collect();
        NameClash::check(path, (source, masked), base, &names)?;
        let undeclared: Vec<String> = names
            .into_iter()
            .filter(|name| !OwnMethods::declares(masked, name))
            .collect();
        let overloads = DynamicDispatch::overloads(path, (source, masked), base, &methods)?;
        let dynamic = ForwardText::written(path, (source, masked), base, (&undeclared, &overloads));
        dynamic.map_or_else(
            || TypedForward::written(path, (source, masked), base, &methods),
            |text| Ok(Some(text)),
        )
    }

    fn in_folder(path: &Path, folder: &str) -> bool {
        Self::REALMS
            .iter()
            .any(|realm| path.starts_with(Path::new(realm).join(folder)))
    }

    fn same_unit(delegate: &Path, service: &Path) -> bool {
        let realm = delegate.components().next() == service.components().next();
        let language = delegate.extension() == service.extension();
        [realm, language] == [true, true]
    }

    fn class_name(path: &Path) -> String {
        path.file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}
