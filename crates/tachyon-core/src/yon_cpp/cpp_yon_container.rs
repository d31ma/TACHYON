use super::{CppYonGraph, CppYonProvider};
use crate::tac_cpp::CppListener;
use crate::yon::plan::Scope;
use serde_json::json;
use std::fmt::Write as _;

pub(crate) struct CppYonContainer;

impl CppYonContainer {
    const START: &'static str = "#line 1 \"tachyon-yon-container.cpp\"\n";

    pub(crate) fn declarations(graph: &CppYonGraph) -> String {
        let (classes, needs) = Self::declared(graph);
        format!(
            "{}class TachyonYonRoutes;\n{classes}namespace tachyon::yon {{\n{needs}}}\n",
            Self::START
        )
    }

    fn declared(graph: &CppYonGraph) -> (String, String) {
        let mut classes = String::new();
        let mut needs = String::new();
        for provider in &graph.providers {
            let _ = writeln!(classes, "class {};", provider.name);
            let _ = writeln!(
                needs,
                "template <>\n::{0}& YonNeeds::of<::{0}>();",
                provider.name
            );
        }
        (classes, needs)
    }

    pub(crate) fn definitions(graph: &CppYonGraph) -> String {
        let made: String = graph.providers.iter().map(Self::definition).collect();
        format!("{}namespace tachyon::yon {{\n{made}}}\n", Self::START)
    }

    fn definition(provider: &CppYonProvider) -> String {
        let name = &provider.name;
        let describe = Self::describe(provider);
        let shared = format!(
            "static ::{name}* slot = nullptr;\n    return YonContainer::shared<::{name}>(slot, {describe});"
        );
        let fresh = format!("return YonContainer::fresh<::{name}>({describe});");
        let made = [shared, fresh][usize::from(provider.scope == Scope::Transient)].clone();
        format!("template <>\n::{name}& YonNeeds::of<::{name}>() {{\n    {made}\n}}\n")
    }

    fn describe(provider: &CppYonProvider) -> String {
        let hooks = &provider.hooks;
        let parts = [
            Self::listening(provider),
            Self::calling("start", &hooks.starts),
            Self::calling("stop", &hooks.stops),
        ];
        let written = format!(
            "[](::{}& owner, YonMade& made) {{\n{}    }}",
            provider.name,
            parts.concat()
        );
        let nothing = format!("[](::{}&, YonMade&) {{}}", provider.name);
        [written, nothing][usize::from(hooks.is_empty())].clone()
    }

    fn listening(provider: &CppYonProvider) -> String {
        let listeners: String = provider
            .hooks
            .listeners
            .iter()
            .map(|listener| Self::listener(&provider.name, listener))
            .collect();
        let empty = provider.hooks.listeners.is_empty();
        let written = format!(
            "        made.listen = [&owner](YonEvents& events) {{\n{listeners}        }};\n"
        );
        [written, String::new()][usize::from(empty)].clone()
    }

    fn listener(owner: &str, listener: &CppListener) -> String {
        let options = json!({
            "events": [listener.event],
            "order": listener.order,
            "condition": listener.condition,
            "async": listener.background,
            "returns": listener.returns,
            "replay": listener.replay,
            "key": format!("{owner}.{}", listener.method),
        });
        format!(
            "            events.listen(R\"tachyon({options})tachyon\", [&owner](const json::JsonNode& value) {{ return YonListener::call(owner, &::{owner}::{}, value); }});\n",
            listener.method
        )
    }

    fn calling(slot: &str, methods: &[String]) -> String {
        let calls = methods.iter().fold(String::new(), |mut calls, method| {
            let _ = write!(calls, " owner.{method}();");
            calls
        });
        let written = format!("        made.{slot} = [&owner] {{{calls} }};\n");
        [written, String::new()][usize::from(methods.is_empty())].clone()
    }
}
