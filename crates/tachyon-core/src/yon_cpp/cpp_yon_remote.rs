use super::{CppYonController, CppYonGateway};
use crate::Failure;
use crate::tac_cpp::{CppFailure, CppRemote, CppService};
use std::path::Path;

pub(crate) struct CppYonRemote;

impl CppYonRemote {
    const SETUPS: &'static str = "server/setups";
    const CLIENT_SUFFIX: &'static str = "Client";
    const SETUP_SUFFIX: &'static str = "Setup";

    pub(crate) fn members(service: &CppService) -> String {
        Self::sends(service)
            .then(|| {
                format!(
                    " ::tachyon::BehaviorContext tachyonBehaviors = ::tachyon::yon::YonRemote::open<{}>({});",
                    Self::setup(service),
                    CppRemote::literal(&CppRemote::adapter(service.class()))
                )
            })
            .into_iter()
            .collect()
    }

    pub(crate) fn definitions(service: &CppService) -> String {
        service
            .is_remote()
            .then(|| {
                format!(
                    "\n{}{}",
                    CppRemote::yon_definitions(service.class()),
                    CppYonGateway::definitions(service.class())
                )
            })
            .into_iter()
            .collect()
    }

    pub(crate) fn check(root: &Path, service: &CppService) -> Result<(), Failure> {
        let setup = Self::setup(service);
        let found = root
            .join(Self::SETUPS)
            .join(format!("{setup}.hpp"))
            .is_file();
        let refused = [Self::sends(service), !found] == [true, true];
        refused
            .then(|| Self::missing(service, &setup))
            .map_or(Ok(()), Err)
    }

    fn sends(service: &CppService) -> bool {
        let requests = CppRemote::requests(service.class()).next().is_some();
        [service.is_remote(), requests] == [true, true]
    }

    fn setup(service: &CppService) -> String {
        let name = &service.class().name;
        let stem = name.strip_suffix(Self::CLIENT_SUFFIX).unwrap_or(name);
        format!("{stem}{}", Self::SETUP_SUFFIX)
    }

    fn missing(service: &CppService, setup: &str) -> Failure {
        CppFailure::coded(
            CppYonController::CODE,
            &service.path,
            service.class().name_at,
            &format!(
                "A Yon C++ [[HTTPClient]] sends its requests through the HTTP adapters of its [[Setup]], and there is no {}/{setup}.hpp. Write `class [[Setup]] {setup}` there with `static BehaviorAdapters configure(const BehaviorExecution& execution)`, which returns adapters.http[\"{}\"]",
                Self::SETUPS,
                CppRemote::adapter(service.class())
            ),
        )
    }
}
