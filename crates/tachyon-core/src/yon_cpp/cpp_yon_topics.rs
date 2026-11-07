use super::{CppYonGraph, CppYonProvider};
use crate::events::TopicUse;

pub(crate) struct CppYonTopics;

impl CppYonTopics {
    const START: &'static str = "#line 1 \"tachyon-yon-topics.cpp\"\n";

    pub(crate) fn source(topics: &[String]) -> String {
        let names: Vec<String> = topics.iter().map(|topic| format!("\"{topic}\"")).collect();
        format!(
            "{}inline const std::vector<std::string>& tachyon::yon::YonTopics::names() {{\n    static const std::vector<std::string> names{{{}}};\n    return names;\n}}\n",
            Self::START,
            names.join(", ")
        )
    }

    pub(crate) fn used(topics: Vec<String>, graph: &CppYonGraph) -> TopicUse {
        let listeners = graph
            .providers
            .iter()
            .flat_map(|provider| Self::listening(provider, &topics))
            .collect();
        TopicUse { topics, listeners }
    }

    fn listening(provider: &CppYonProvider, topics: &[String]) -> Vec<(String, String)> {
        provider
            .hooks
            .listeners
            .iter()
            .filter(|listener| topics.contains(&listener.event))
            .map(|listener| {
                (
                    listener.event.clone(),
                    format!("{}.{}", provider.name, listener.method),
                )
            })
            .collect()
    }
}
