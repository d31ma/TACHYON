use super::registry::NativeCompanionInput;
use crate::Failure;
use crate::delegate_forwards::DelegateForwards;
use crate::failure::{diagnostic, source_span};
use crate::project::NativeCompanion;
use std::path::{Path, PathBuf};

pub(super) struct WebDelegateBases;

impl WebDelegateBases {
    const CODE: u16 = 1406;
    const SERVICES: &'static str = "client/services";
    const DELEGATES: &'static str = "client/delegates";
    const RUST: &'static str = "rs";
    const HELP: &'static str = "Let the view call the service, or keep this Delegate without a service field: a Delegate without a base works in a Rust browser build. A native Rust build shares one page graph, so a Delegate there can take on a service (ADR 0112).";

    pub(super) fn refuse(selected: &[&NativeCompanionInput]) -> Result<(), Failure> {
        selected
            .iter()
            .find(|item| {
                [item.language == NativeCompanion::Rust, item.web_assembly] == [true, true]
            })
            .map_or(Ok(()), |item| Self::refuse_in(&item.snapshot_root))
    }

    fn refuse_in(root: &Path) -> Result<(), Failure> {
        let services = Self::files(root, Self::SERVICES);
        let forwards = DelegateForwards::new(
            services
                .iter()
                .map(|(path, source)| (path.as_path(), source.as_bytes())),
        );
        let delegates = Self::files(root, Self::DELEGATES);
        let taken = delegates.iter().find_map(|(path, source)| {
            let base = forwards.taken_on(path, source)?;
            Some((path, source, base.service))
        });
        taken.map_or(Ok(()), |(path, source, service)| {
            Err(Self::failure(path, source, &service))
        })
    }

    fn files(root: &Path, folder: &str) -> Vec<(PathBuf, String)> {
        std::fs::read_dir(root.join(folder))
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == Self::RUST)
            })
            .filter_map(|path| {
                let source = std::fs::read_to_string(&path).ok()?;
                Some((Path::new(folder).join(path.file_name()?), source))
            })
            .collect()
    }

    fn failure(path: &Path, source: &str, service: &str) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        let delegate = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let start = source.find(service).unwrap_or_default();
        Failure::one(diagnostic(
            Self::CODE,
            format!(
                "{delegate} takes on {service}, but a Rust browser service cannot need another service: the Delegate's field would hold its own copy of {service}, not the one that the page uses (ADR 0065)."
            ),
            Some(Self::HELP.to_owned()),
            source_span(&portable, start, start + service.len()),
        ))
    }
}
