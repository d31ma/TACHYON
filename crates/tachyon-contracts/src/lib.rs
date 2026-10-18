mod artifact_contract_versions;
mod artifact_manifest;
mod artifact_output;
mod artifact_target;
mod artifact_toolchain;
mod capability_grant;
mod capability_manifest;
mod collision_policy;
mod counted_comparison;
mod handler_body;
mod handler_body_encoding;
mod handler_cancel;
mod handler_cancel_kind;
mod handler_envelope;
mod handler_protocol_error;
mod handler_request;
mod handler_request_kind;
mod handler_response;
mod handler_response_kind;
mod handler_violation;
mod hot_update;
mod hot_update_kind;
mod http_method;
mod native_accessibility;
mod native_node;
mod native_target;
mod native_ui;
mod route_context;
mod route_entry;
mod route_handler;
mod route_kind;
mod route_manifest;
mod view_ir;
mod view_node;
mod view_source_map;
mod view_source_mapping;
mod web_surface_bridge;
mod web_surface_source;

pub use artifact_contract_versions::ArtifactContractVersions;
pub use artifact_manifest::ArtifactManifest;
pub use artifact_output::ArtifactOutput;
pub use artifact_target::ArtifactTarget;
pub use artifact_toolchain::ArtifactToolchain;
pub use capability_grant::CapabilityGrant;
pub use capability_manifest::CapabilityManifest;
pub use collision_policy::CollisionPolicy;
pub use counted_comparison::CountedComparison;
pub use handler_body::HandlerBody;
pub use handler_body_encoding::HandlerBodyEncoding;
pub use handler_cancel::HandlerCancel;
pub use handler_cancel_kind::HandlerCancelKind;
pub use handler_envelope::HandlerEnvelope;
pub use handler_protocol_error::HandlerProtocolError;
pub use handler_request::HandlerRequest;
pub use handler_request_kind::HandlerRequestKind;
pub use handler_response::HandlerResponse;
pub use handler_response_kind::HandlerResponseKind;
pub use handler_violation::HandlerViolation;
pub use hot_update::HotUpdate;
pub use hot_update_kind::HotUpdateKind;
pub use http_method::HttpMethod;
pub use native_accessibility::NativeAccessibility;
pub use native_node::NativeNode;
pub use native_target::NativeTarget;
pub use native_ui::NativeUi;
pub use route_context::RouteContext;
pub use route_entry::RouteEntry;
pub use route_handler::RouteHandler;
pub use route_kind::RouteKind;
pub use route_manifest::RouteManifest;
pub use view_ir::ViewIr;
pub use view_node::ViewNode;
pub use view_source_map::ViewSourceMap;
pub use view_source_mapping::ViewSourceMapping;
pub use web_surface_bridge::WebSurfaceBridge;
pub use web_surface_source::WebSurfaceSource;

use std::collections::BTreeMap;

use serde_json::Value;

pub const PRODUCT_VERSION: &str = env!("TAC_PRODUCT_VERSION");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Contract {
    pub name: &'static str,
    pub id: &'static str,
    pub major_version: u16,
    pub schema: &'static str,
    pub valid_example: &'static str,
    pub invalid_example: &'static str,
}

impl Contract {
    pub const JSON_SCHEMA_DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";
}

macro_rules! contract {
    ($name:literal, $version:literal) => {
        Contract {
            name: $name,
            id: concat!("https://tachyon.del.ma/schema/", $name, "/v", $version),
            major_version: $version,
            schema: include_str!(concat!(
                "../../../api/",
                $name,
                "/v",
                $version,
                "/schema.json"
            )),
            valid_example: include_str!(concat!(
                "../../../api/",
                $name,
                "/v",
                $version,
                "/examples/valid.json"
            )),
            invalid_example: include_str!(concat!(
                "../../../api/",
                $name,
                "/v",
                $version,
                "/examples/invalid.json"
            )),
        }
    };
}

pub const CONTRACTS: [Contract; 12] = [
    contract!("annotation-modules", 1),
    contract!("application-config", 3),
    contract!("artifact-manifest", 1),
    contract!("capability-manifest", 1),
    contract!("diagnostics", 1),
    contract!("handler-protocol", 1),
    contract!("hot-update", 1),
    contract!("native-host", 3),
    contract!("native-ui", 1),
    contract!("route-manifest", 1),
    contract!("view-ir", 1),
    contract!("view-source-map", 1),
];

pub type HandlerHeaders = BTreeMap<String, Vec<String>>;

#[must_use]
pub fn find(name: &str) -> Option<&'static Contract> {
    CONTRACTS.iter().find(|contract| contract.name == name)
}

pub fn parse_schema(contract: &Contract) -> Result<Value, serde_json::Error> {
    serde_json::from_str(contract.schema)
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::{
        ArtifactContractVersions, ArtifactManifest, ArtifactOutput, ArtifactTarget,
        ArtifactToolchain, CONTRACTS, CapabilityManifest, CollisionPolicy, Contract, HandlerBody,
        HandlerBodyEncoding, HandlerCancel, HandlerEnvelope, HandlerHeaders, HandlerProtocolError,
        HandlerRequest, HandlerResponse, HotUpdate, HotUpdateKind, HttpMethod, NativeAccessibility,
        NativeNode, NativeTarget, NativeUi, RouteContext, RouteEntry, RouteHandler, RouteKind,
        RouteManifest, WebSurfaceBridge, WebSurfaceSource, find, parse_schema,
    };
    use std::collections::BTreeMap;

    #[test]
    fn every_schema_is_canonical_and_examples_prove_both_paths()
    -> Result<(), Box<dyn std::error::Error>> {
        for contract in &CONTRACTS {
            let schema = parse_schema(contract)?;
            jsonschema::meta::validate(&schema).map_err(|error| {
                io::Error::other(format!(
                    "{} does not satisfy its JSON meta-schema: {error}",
                    contract.name
                ))
            })?;

            assert_eq!(
                schema["$schema"],
                Contract::JSON_SCHEMA_DRAFT,
                "{}",
                contract.name
            );
            assert_eq!(schema["$id"], contract.id, "{}", contract.name);

            let validator = jsonschema::validator_for(&schema).map_err(|error| {
                io::Error::other(format!("{} could not compile: {error}", contract.name))
            })?;
            let valid = serde_json::from_str(contract.valid_example)?;
            let invalid = serde_json::from_str(contract.invalid_example)?;

            assert!(validator.is_valid(&valid), "{}", contract.name);
            assert!(!validator.is_valid(&invalid), "{}", contract.name);
        }

        Ok(())
    }

    #[test]
    fn registry_names_and_ids_are_unique_and_ordered() {
        let mut names = CONTRACTS
            .iter()
            .map(|contract| contract.name)
            .collect::<Vec<_>>();
        let original_names = names.clone();
        names.sort_unstable();
        names.dedup();

        assert_eq!(names, original_names);

        let mut ids = CONTRACTS
            .iter()
            .map(|contract| contract.id)
            .collect::<Vec<_>>();
        let original_len = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), original_len);
    }

    #[test]
    fn contracts_are_discoverable_by_stable_name() {
        assert_eq!(
            find("view-ir").map(|contract| contract.major_version),
            Some(1)
        );
        assert_eq!(
            find("hot-update").map(|contract| contract.major_version),
            Some(1)
        );
        assert!(find("unknown").is_none());
    }

    #[test]
    fn hot_update_has_the_canonical_wire_shape() -> Result<(), Box<dyn std::error::Error>> {
        let update = HotUpdate::v1(
            7,
            HotUpdateKind::Island,
            Some(String::from(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            )),
            vec![String::from("client/components/product-card/tac.js")],
            vec![String::from("product-card")],
            None,
        );
        let value = serde_json::to_value(&update)?;
        let contract = find("hot-update").ok_or_else(|| io::Error::other("missing contract"))?;
        let validator = jsonschema::validator_for(&parse_schema(contract)?)?;

        assert!(validator.is_valid(&value));
        assert_eq!(value["kind"], "island");
        assert_eq!(serde_json::from_value::<HotUpdate>(value)?, update);

        Ok(())
    }

    #[test]
    fn route_manifest_types_have_the_canonical_wire_shape() -> Result<(), serde_json::Error> {
        assert_api_route_wire_shape()?;
        assert_page_route_wire_shape()
    }

    fn assert_api_route_wire_shape() -> Result<(), serde_json::Error> {
        let methods = vec![
            HttpMethod::Delete,
            HttpMethod::Get,
            HttpMethod::Head,
            HttpMethod::Options,
            HttpMethod::Patch,
            HttpMethod::Post,
            HttpMethod::Put,
        ];
        let manifest = RouteManifest::v1(vec![RouteEntry {
            route: String::from("/api"),
            parameters: Vec::new(),
            kind: RouteKind::Api,
            methods,
            view: None,
            handlers: vec![RouteHandler {
                source: String::from("server/routes/api/yon.js"),
                language: String::from("javascript"),
                runtime: String::from("bun"),
            }],
            context: RouteContext::default(),
        }]);
        let value = serde_json::to_value(manifest)?;
        assert_eq!(value["contract_version"], 1);
        assert_eq!(value["routes"][0]["kind"], "api");
        assert_eq!(value["routes"][0]["methods"][0], "DELETE");
        assert_eq!(value["routes"][0]["methods"][6], "PUT");
        assert_eq!(value["routes"][0]["context"]["collision_policy"], "error");
        assert_eq!(
            value["routes"][0]["context"]["static_exports"],
            serde_json::json!([])
        );
        assert_eq!(
            value["routes"][0]["context"]["response_exports"],
            serde_json::json!([])
        );
        assert_eq!(
            RouteContext::default().collision_policy,
            CollisionPolicy::Error
        );
        Ok(())
    }

    fn assert_page_route_wire_shape() -> Result<(), serde_json::Error> {
        let page = serde_json::to_value(RouteManifest::v1(vec![RouteEntry {
            route: String::from("/"),
            parameters: Vec::new(),
            kind: RouteKind::Page,
            methods: vec![HttpMethod::Get],
            view: Some(String::from("client/pages/tac.html")),
            handlers: Vec::new(),
            context: RouteContext::default(),
        }]))?;
        assert_eq!(page["routes"][0]["kind"], "page");
        Ok(())
    }

    #[test]
    fn native_and_packaging_types_have_canonical_wire_shapes() -> Result<(), serde_json::Error> {
        assert_native_wire_shape()?;
        assert_capability_wire_shape()?;
        assert_artifact_wire_shape()
    }

    fn assert_native_wire_shape() -> Result<(), serde_json::Error> {
        let native = NativeUi::v1(
            NativeTarget::Macos,
            NativeNode::NativeElement {
                id: Some(String::from("n_root")),
                adapter: String::from("layout.column"),
                properties: BTreeMap::new(),
                events: BTreeMap::new(),
                accessibility: Some(NativeAccessibility {
                    role: Some(String::from("main")),
                    label: Some(String::from("Catalog")),
                }),
                children: vec![NativeNode::WebSurface {
                    id: Some(String::from("n_chart")),
                    source: WebSurfaceSource::LocalBundle,
                    location: String::from("WebSurfaces/n_chart/index.html"),
                    bridge: WebSurfaceBridge::None,
                    reason: String::from("No native chart adapter."),
                    accessibility: Some(NativeAccessibility {
                        role: Some(String::from("group")),
                        label: Some(String::from("Chart")),
                    }),
                }],
            },
        );
        let native_value = serde_json::to_value(native)?;
        assert_eq!(native_value["target"], "macos");
        assert_eq!(native_value["root"]["kind"], "native_element");
        assert_eq!(native_value["root"]["children"][0]["bridge"], "none");
        Ok(())
    }

    fn assert_capability_wire_shape() -> Result<(), serde_json::Error> {
        let capability = CapabilityManifest::deny_all(String::from("dev.tachyon.catalog"));
        let capability_value = serde_json::to_value(capability)?;
        assert_eq!(capability_value["default_policy"], "deny");
        assert_eq!(capability_value["remote_content_bridge"], false);
        Ok(())
    }

    fn assert_artifact_wire_shape() -> Result<(), serde_json::Error> {
        let artifact = ArtifactManifest {
            contract_version: 1,
            release_version: String::from("0.0.0-phase4"),
            commit: "0".repeat(40),
            source_date_epoch: 0,
            target: ArtifactTarget {
                os: String::from("macos"),
                architecture: String::from("aarch64"),
                abi: String::from("swiftui"),
            },
            toolchains: vec![ArtifactToolchain {
                name: String::from("swift"),
                version: String::from("6.3.3"),
            }],
            contracts: ArtifactContractVersions::default(),
            outputs: vec![ArtifactOutput {
                path: String::from("PhaseFour.app/Contents/MacOS/PhaseFour"),
                sha256: "0".repeat(64),
                size: 1,
            }],
        };
        let artifact_value = serde_json::to_value(artifact)?;
        assert_eq!(artifact_value["contracts"]["native_host"], 3);
        assert!(artifact_value["contracts"].get("native_ui").is_none());
        assert!(
            artifact_value["contracts"]
                .get("capability_manifest")
                .is_none()
        );
        assert_eq!(artifact_value["target"]["os"], "macos");
        Ok(())
    }

    #[test]
    fn handler_protocol_types_cover_every_canonical_envelope() -> Result<(), serde_json::Error> {
        assert_request_envelope()?;
        assert_response_envelope()?;
        assert_failure_envelope()?;
        assert_cancel_envelope()
    }

    fn assert_request_envelope() -> Result<(), serde_json::Error> {
        let mut request = HandlerRequest::route("req_01", "/products", HttpMethod::Post);
        request.deadline_ms = Some(2_000);
        request.headers.insert(
            String::from("content-type"),
            vec![String::from("application/json")],
        );
        request.body = Some(HandlerBody {
            encoding: HandlerBodyEncoding::Utf8,
            data: String::from(r#"{"name":"Ada"}"#),
        });
        let request_value = serde_json::to_value(HandlerEnvelope::Request(request.clone()))?;
        assert_eq!(request_value["kind"], "request");
        assert_eq!(request_value["method"], "POST");
        assert_eq!(
            serde_json::from_value::<HandlerEnvelope>(request_value)?,
            HandlerEnvelope::Request(request)
        );
        Ok(())
    }

    fn assert_response_envelope() -> Result<(), serde_json::Error> {
        let mut headers = HandlerHeaders::new();
        headers.insert(
            String::from("content-type"),
            vec![String::from("application/json; charset=utf-8")],
        );
        let response = HandlerResponse::success(
            "req_01",
            200,
            headers,
            HandlerBody {
                encoding: HandlerBodyEncoding::Utf8,
                data: String::from(r#"{"ok":true}"#),
            },
        );
        let response_value = serde_json::to_value(HandlerEnvelope::Response(response.clone()))?;
        assert_eq!(response_value["kind"], "response");
        assert!(response_value.get("error").is_none());
        assert_eq!(
            serde_json::from_value::<HandlerEnvelope>(response_value)?,
            HandlerEnvelope::Response(response)
        );
        Ok(())
    }

    fn assert_failure_envelope() -> Result<(), serde_json::Error> {
        let failure = HandlerResponse::error(
            "req_02",
            500,
            HandlerProtocolError::new("TY2201", "Handler failed."),
        );
        let failure_value = serde_json::to_value(failure)?;
        assert!(failure_value.get("body").is_none());
        assert_eq!(failure_value["error"]["code"], "TY2201");
        Ok(())
    }

    fn assert_cancel_envelope() -> Result<(), serde_json::Error> {
        let cancel = HandlerCancel::v1("req_01");
        let cancel_value = serde_json::to_value(HandlerEnvelope::Cancel(cancel.clone()))?;
        assert_eq!(cancel_value["kind"], "cancel");
        assert_eq!(
            serde_json::from_value::<HandlerEnvelope>(cancel_value)?,
            HandlerEnvelope::Cancel(cancel)
        );
        Ok(())
    }
}
