use super::rust_service_plan::RustServicePlan;
use crate::Failure;

pub(super) struct NativeApplicationCapture;

impl NativeApplicationCapture {
    pub(super) fn validate(service: &RustServicePlan) -> Result<(), Failure> {
        Self::validated(service)
    }

    pub(super) fn projection(service: &RustServicePlan, exposed: bool) -> Result<(), Failure> {
        Self::page_projection(service, exposed)
    }

    pub(super) fn dependencies(
        service: &RustServicePlan,
        services: &std::collections::BTreeMap<String, RustServicePlan>,
        dependencies: &std::collections::BTreeSet<String>,
    ) -> Result<(), Failure> {
        let shorter = Self::shorter(services, dependencies);
        Self::compatible(service, shorter)
    }

    fn validated(service: &RustServicePlan) -> Result<(), Failure> {
        service
            .application
            .then(|| Self::validate_app(service))
            .transpose()
            .map(|_| ())
    }

    fn validate_app(service: &RustServicePlan) -> Result<(), Failure> {
        Self::source(service)?;
        service.refuse_unowned_events()
    }

    fn page_projection(service: &RustServicePlan, exposed: bool) -> Result<(), Failure> {
        ([service.application, exposed] != [true; 2]).then_some(()).ok_or_else(|| service.failure(
            "An application Service is a provider, not a directly view-bound Service. Inject Arc<ProviderService> into a fresh page Service."))
    }

    fn shorter(
        services: &std::collections::BTreeMap<String, RustServicePlan>,
        dependencies: &std::collections::BTreeSet<String>,
    ) -> bool {
        dependencies
            .iter()
            .filter_map(|name| services.get(name))
            .any(|dependency| !dependency.application)
    }

    fn compatible(service: &RustServicePlan, shorter: bool) -> Result<(), Failure> {
        ([service.application, shorter] != [true; 2]).then_some(()).ok_or_else(|| service.failure(
            "An application Service cannot capture a page or prototype Service. Mark a genuine application-compatible provider explicitly, or keep ordinary handwritten ownership."))
    }

    fn source(service: &RustServicePlan) -> Result<(), Failure> {
        let masked = crate::annotation_layout::masked_source(&service.path, &service.source)?;
        Self::require_independent(service, &masked)
    }

    fn require_independent(service: &RustServicePlan, source: &str) -> Result<(), Failure> {
        let captures = source
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .any(Self::page_provider);
        (!captures).then_some(()).ok_or_else(|| service.failure(
            "An application Service cannot use page-scoped synthetic providers or activation-owned capabilities, including ApplicationEventPublisher, FilePicker and WindowControls. Keep those providers in a fresh page Service."))
    }

    fn page_provider(name: &str) -> bool {
        [
            name == "ApplicationEventPublisher",
            crate::web_api_clients::WebApiClients::is_client_type(name),
        ]
        .contains(&true)
    }
}
