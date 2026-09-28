use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
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

pub const CONTRACTS: [Contract; 12] = [
    Contract {
        name: "annotation-modules",
        id: "https://tachyon.del.ma/schema/annotation-modules/v1",
        major_version: 1,
        schema: include_str!("../../../api/annotation-modules/v1/schema.json"),
        valid_example: include_str!("../../../api/annotation-modules/v1/examples/valid.json"),
        invalid_example: include_str!("../../../api/annotation-modules/v1/examples/invalid.json"),
    },
    Contract {
        name: "application-config",
        id: "https://tachyon.del.ma/schema/application-config/v3",
        major_version: 3,
        schema: include_str!("../../../api/application-config/v3/schema.json"),
        valid_example: include_str!("../../../api/application-config/v3/examples/valid.json"),
        invalid_example: include_str!("../../../api/application-config/v3/examples/invalid.json"),
    },
    Contract {
        name: "artifact-manifest",
        id: "https://tachyon.del.ma/schema/artifact-manifest/v1",
        major_version: 1,
        schema: include_str!("../../../api/artifact-manifest/v1/schema.json"),
        valid_example: include_str!("../../../api/artifact-manifest/v1/examples/valid.json"),
        invalid_example: include_str!("../../../api/artifact-manifest/v1/examples/invalid.json"),
    },
    Contract {
        name: "capability-manifest",
        id: "https://tachyon.del.ma/schema/capability-manifest/v1",
        major_version: 1,
        schema: include_str!("../../../api/capability-manifest/v1/schema.json"),
        valid_example: include_str!("../../../api/capability-manifest/v1/examples/valid.json"),
        invalid_example: include_str!("../../../api/capability-manifest/v1/examples/invalid.json"),
    },
    Contract {
        name: "diagnostics",
        id: "https://tachyon.del.ma/schema/diagnostics/v1",
        major_version: 1,
        schema: include_str!("../../../api/diagnostics/v1/schema.json"),
        valid_example: include_str!("../../../api/diagnostics/v1/examples/valid.json"),
        invalid_example: include_str!("../../../api/diagnostics/v1/examples/invalid.json"),
    },
    Contract {
        name: "handler-protocol",
        id: "https://tachyon.del.ma/schema/handler-protocol/v1",
        major_version: 1,
        schema: include_str!("../../../api/handler-protocol/v1/schema.json"),
        valid_example: include_str!("../../../api/handler-protocol/v1/examples/valid.json"),
        invalid_example: include_str!("../../../api/handler-protocol/v1/examples/invalid.json"),
    },
    Contract {
        name: "hot-update",
        id: "https://tachyon.del.ma/schema/hot-update/v1",
        major_version: 1,
        schema: include_str!("../../../api/hot-update/v1/schema.json"),
        valid_example: include_str!("../../../api/hot-update/v1/examples/valid.json"),
        invalid_example: include_str!("../../../api/hot-update/v1/examples/invalid.json"),
    },
    Contract {
        name: "native-host",
        id: "https://tachyon.del.ma/schema/native-host/v3",
        major_version: 3,
        schema: include_str!("../../../api/native-host/v3/schema.json"),
        valid_example: include_str!("../../../api/native-host/v3/examples/valid.json"),
        invalid_example: include_str!("../../../api/native-host/v3/examples/invalid.json"),
    },
    Contract {
        name: "native-ui",
        id: "https://tachyon.del.ma/schema/native-ui/v1",
        major_version: 1,
        schema: include_str!("../../../api/native-ui/v1/schema.json"),
        valid_example: include_str!("../../../api/native-ui/v1/examples/valid.json"),
        invalid_example: include_str!("../../../api/native-ui/v1/examples/invalid.json"),
    },
    Contract {
        name: "route-manifest",
        id: "https://tachyon.del.ma/schema/route-manifest/v1",
        major_version: 1,
        schema: include_str!("../../../api/route-manifest/v1/schema.json"),
        valid_example: include_str!("../../../api/route-manifest/v1/examples/valid.json"),
        invalid_example: include_str!("../../../api/route-manifest/v1/examples/invalid.json"),
    },
    Contract {
        name: "view-ir",
        id: "https://tachyon.del.ma/schema/view-ir/v1",
        major_version: 1,
        schema: include_str!("../../../api/view-ir/v1/schema.json"),
        valid_example: include_str!("../../../api/view-ir/v1/examples/valid.json"),
        invalid_example: include_str!("../../../api/view-ir/v1/examples/invalid.json"),
    },
    Contract {
        name: "view-source-map",
        id: "https://tachyon.del.ma/schema/view-source-map/v1",
        major_version: 1,
        schema: include_str!("../../../api/view-source-map/v1/schema.json"),
        valid_example: include_str!("../../../api/view-source-map/v1/examples/valid.json"),
        invalid_example: include_str!("../../../api/view-source-map/v1/examples/invalid.json"),
    },
];

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HotUpdate {
    pub contract_version: u8,
    pub sequence: u64,
    pub kind: HotUpdateKind,
    pub build_id: Option<String>,
    pub paths: Vec<String>,
    pub boundaries: Vec<String>,
    pub diagnostics: Option<tachyon_diagnostics::DiagnosticReport>,
}

impl HotUpdate {
    #[must_use]
    pub const fn v1(
        sequence: u64,
        kind: HotUpdateKind,
        build_id: Option<String>,
        paths: Vec<String>,
        boundaries: Vec<String>,
        diagnostics: Option<tachyon_diagnostics::DiagnosticReport>,
    ) -> Self {
        Self {
            contract_version: 1,
            sequence,
            kind,
            build_id,
            paths,
            boundaries,
            diagnostics,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotUpdateKind {
    Css,
    Island,
    Reload,
    Diagnostics,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewIr {
    pub contract_version: u8,
    pub source: String,
    pub root: ViewNode,
}

impl ViewIr {
    #[must_use]
    pub const fn v1(source: String, root: ViewNode) -> Self {
        Self {
            contract_version: 1,
            source,
            root,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ViewNode {
    Element {
        tag: String,
        attributes: BTreeMap<String, String>,
        children: Vec<Self>,
    },
    Text {
        value: String,
    },
    Conditional {
        condition: String,
        then: Vec<Self>,
        #[serde(rename = "else")]
        otherwise: Vec<Self>,
    },
    Iteration {
        binding: String,
        iterable: String,
        body: Vec<Self>,
        empty: Vec<Self>,
    },
    CountedIteration {
        binding: String,
        from: String,
        comparison: CountedComparison,
        to: String,
        step: String,
        body: Vec<Self>,
    },
    Component {
        name: String,
        properties: BTreeMap<String, String>,
        children: Vec<Self>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CountedComparison {
    Lt,
    Le,
    Gt,
    Ge,
}

impl CountedComparison {
    #[must_use]
    pub const fn ascending(self) -> bool {
        matches!(self, Self::Lt | Self::Le)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewSourceMap {
    pub contract_version: u8,
    pub output: String,
    pub sources: Vec<String>,
    pub mappings: Vec<ViewSourceMapping>,
}

impl ViewSourceMap {
    #[must_use]
    pub const fn v1(
        output: String,
        sources: Vec<String>,
        mappings: Vec<ViewSourceMapping>,
    ) -> Self {
        Self {
            contract_version: 1,
            output,
            sources,
            mappings,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewSourceMapping {
    pub generated_start: u64,
    pub generated_end: u64,
    pub source: String,
    pub source_start: u64,
    pub source_end: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NativeTarget {
    Linux,
    Macos,
    Windows,
    Android,
    Ios,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeUi {
    pub contract_version: u8,
    pub target: NativeTarget,
    pub root: NativeNode,
}

impl NativeUi {
    #[must_use]
    pub const fn v1(target: NativeTarget, root: NativeNode) -> Self {
        Self {
            contract_version: 1,
            target,
            root,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAccessibility {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSurfaceSource {
    LocalBundle,
    RemoteUrl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSurfaceBridge {
    None,
    LocalCapabilities,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NativeNode {
    NativeElement {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        adapter: String,
        properties: BTreeMap<String, Value>,
        events: BTreeMap<String, String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        accessibility: Option<NativeAccessibility>,
        children: Vec<Self>,
    },
    Text {
        value: String,
    },
    WebSurface {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        source: WebSurfaceSource,
        location: String,
        bridge: WebSurfaceBridge,
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        accessibility: Option<NativeAccessibility>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityManifest {
    pub contract_version: u8,
    pub application_id: String,
    pub default_policy: String,
    pub remote_content_bridge: bool,
    pub capabilities: Vec<CapabilityGrant>,
}

impl CapabilityManifest {
    #[must_use]
    pub fn deny_all(application_id: String) -> Self {
        Self {
            contract_version: 1,
            application_id,
            default_policy: String::from("deny"),
            remote_content_bridge: false,
            capabilities: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityGrant {
    pub name: String,
    pub scope: Vec<String>,
    pub reason: String,
    pub targets: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactManifest {
    pub contract_version: u8,
    pub release_version: String,
    pub commit: String,
    pub source_date_epoch: u64,
    pub target: ArtifactTarget,
    pub toolchains: Vec<ArtifactToolchain>,
    pub contracts: ArtifactContractVersions,
    pub outputs: Vec<ArtifactOutput>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTarget {
    pub os: String,
    pub architecture: String,
    pub abi: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactToolchain {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactContractVersions {
    pub artifact_manifest: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_manifest: Option<u8>,
    pub diagnostics: u8,
    pub handler_protocol: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_ui: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_host: Option<u8>,
    pub route_manifest: u8,
    pub view_ir: u8,
}

impl Default for ArtifactContractVersions {
    fn default() -> Self {
        Self {
            artifact_manifest: 1,
            capability_manifest: None,
            diagnostics: 1,
            handler_protocol: 1,
            native_ui: None,
            native_host: Some(3),
            route_manifest: 1,
            view_ir: 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactOutput {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteManifest {
    pub contract_version: u8,
    pub routes: Vec<RouteEntry>,
}

impl RouteManifest {
    #[must_use]
    pub const fn v1(routes: Vec<RouteEntry>) -> Self {
        Self {
            contract_version: 1,
            routes,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteEntry {
    pub route: String,
    pub parameters: Vec<String>,
    pub kind: RouteKind,
    pub methods: Vec<HttpMethod>,
    pub view: Option<String>,
    pub handlers: Vec<RouteHandler>,
    pub context: RouteContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteKind {
    Page,
    Api,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Delete,
    Get,
    Head,
    Options,
    Patch,
    Post,
    Put,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteHandler {
    pub source: String,
    pub language: String,
    pub runtime: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteContext {
    pub collision_policy: CollisionPolicy,
    pub static_exports: Vec<String>,
    pub response_exports: Vec<String>,
}

impl Default for RouteContext {
    fn default() -> Self {
        Self {
            collision_policy: CollisionPolicy::Error,
            static_exports: Vec::new(),
            response_exports: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollisionPolicy {
    Error,
}

pub type HandlerHeaders = BTreeMap<String, Vec<String>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HandlerBodyEncoding {
    Utf8,
    Base64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlerBody {
    pub encoding: HandlerBodyEncoding,
    pub data: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HandlerRequestKind {
    #[serde(rename = "request")]
    Request,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlerRequest {
    pub protocol_version: u8,
    pub kind: HandlerRequestKind,
    pub request_id: String,
    pub operation: String,
    pub route: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub method: HttpMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline_ms: Option<u64>,
    pub headers: HandlerHeaders,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<HandlerBody>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<serde_json::Value>>,
}

impl HandlerRequest {
    #[must_use]
    pub fn route(
        request_id: impl Into<String>,
        route: impl Into<String>,
        method: HttpMethod,
    ) -> Self {
        Self {
            protocol_version: 1,
            kind: HandlerRequestKind::Request,
            request_id: request_id.into(),
            operation: String::from("route.invoke"),
            route: route.into(),
            target: None,
            method,
            deadline_ms: None,
            headers: HandlerHeaders::new(),
            parameters: BTreeMap::new(),
            body: None,
            arguments: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HandlerResponseKind {
    #[serde(rename = "response")]
    Response,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlerProtocolError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlerResponse {
    pub protocol_version: u8,
    pub kind: HandlerResponseKind,
    pub request_id: String,
    pub status: u16,
    pub headers: HandlerHeaders,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<HandlerBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<HandlerProtocolError>,
}

impl HandlerResponse {
    #[must_use]
    pub fn success(
        request_id: impl Into<String>,
        status: u16,
        headers: HandlerHeaders,
        body: HandlerBody,
    ) -> Self {
        Self {
            protocol_version: 1,
            kind: HandlerResponseKind::Response,
            request_id: request_id.into(),
            status,
            headers,
            body: Some(body),
            error: None,
        }
    }

    #[must_use]
    pub fn error(request_id: impl Into<String>, status: u16, error: HandlerProtocolError) -> Self {
        Self {
            protocol_version: 1,
            kind: HandlerResponseKind::Response,
            request_id: request_id.into(),
            status,
            headers: HandlerHeaders::new(),
            body: None,
            error: Some(error),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HandlerCancelKind {
    #[serde(rename = "cancel")]
    Cancel,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlerCancel {
    pub protocol_version: u8,
    pub kind: HandlerCancelKind,
    pub request_id: String,
}

impl HandlerCancel {
    #[must_use]
    pub fn v1(request_id: impl Into<String>) -> Self {
        Self {
            protocol_version: 1,
            kind: HandlerCancelKind::Cancel,
            request_id: request_id.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HandlerEnvelope {
    Request(HandlerRequest),
    Response(HandlerResponse),
    Cancel(HandlerCancel),
}

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
            HandlerProtocolError {
                code: String::from("TY2201"),
                message: String::from("Handler failed."),
                retryable: false,
            },
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
