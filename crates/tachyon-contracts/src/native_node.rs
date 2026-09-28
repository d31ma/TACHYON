use crate::{NativeAccessibility, WebSurfaceBridge, WebSurfaceSource};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

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
