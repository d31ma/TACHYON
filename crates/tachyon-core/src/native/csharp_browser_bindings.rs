use std::fmt::Write as _;

pub(super) struct CsharpBrowserBindings;

impl CsharpBrowserBindings {
    pub(super) fn read(authored: &str) -> String {
        Self::render(&Self::namespaces(authored))
    }

    fn namespaces(authored: &str) -> Vec<usize> {
        authored
            .split("namespace TachyonService")
            .skip(1)
            .enumerate()
            .filter(|(_, source)| source.contains("internal static class BehaviorBrowserHttp"))
            .map(|(index, _)| index)
            .collect()
    }

    fn render(namespaces: &[usize]) -> String {
        let mut bindings = String::new();
        let mut patterns = String::new();
        for index in namespaces {
            let _ = write!(
                bindings,
                "\n        TachyonService{index}.BehaviorBrowserHttp.Capture ??= Tac.CaptureHttpTransport;"
            );
            let _ = write!(
                patterns,
                "TachyonService{index}.BehaviorError failure{index} => failure{index}.Code, "
            );
        }
        format!(
            "{bindings}\n        TacTaskAnswer.FailureCode ??= error => error switch {{ {patterns}_ => null }};"
        )
    }
}
