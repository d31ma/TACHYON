use std::fmt::Write as _;

pub(super) struct CsharpBrowserBindings;

impl CsharpBrowserBindings {
    #[cfg(test)]
    pub(super) fn read(authored: &str) -> String {
        Self::read_with_runtime(authored, "")
    }

    pub(super) fn read_with_runtime(authored: &str, runtime: &str) -> String {
        let mut namespaces = Self::namespaces(authored);
        namespaces.extend(Self::namespaces(runtime));
        namespaces.sort_unstable();
        namespaces.dedup();
        Self::render(&namespaces)
    }

    fn namespaces(authored: &str) -> Vec<usize> {
        authored
            .split("namespace TachyonService")
            .skip(1)
            .filter(|source| source.contains("internal static class BehaviorBrowserHttp"))
            .filter_map(Self::namespace_index)
            .collect()
    }

    fn namespace_index(source: &str) -> Option<usize> {
        let end = source.find(|character: char| !character.is_ascii_digit())?;
        source[end..]
            .trim_start()
            .starts_with('{')
            .then(|| source[..end].parse().ok())
            .flatten()
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
