use crate::CountedComparison;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
