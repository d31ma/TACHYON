use crate::Failure;
use std::path::Path;

mod base_methods;
mod class_close;
mod delegate_base;
mod forward_text;
mod own_methods;
mod source_lines;
#[cfg(test)]
mod tests;

use base_methods::BaseMethods;
use class_close::ClassClose;
use delegate_base::DelegateBase;
use forward_text::ForwardText;
use own_methods::OwnMethods;
use source_lines::SourceLines;

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
        Ok(base.and_then(|base| Self::written(path, source, &base)))
    }

    fn written(path: &Path, source: &str, base: &DelegateBase<'_>) -> Option<String> {
        let masked = crate::annotation_layout::masked_source(path, source).ok()?;
        let names = BaseMethods::forwarded(base, &masked);
        ForwardText::written(path, (source, &masked), base, &names)
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
