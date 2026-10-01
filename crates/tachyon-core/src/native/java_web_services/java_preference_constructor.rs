use super::java_code::JavaCode;
use super::java_declaration::JavaDeclaration;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(super) struct JavaPreferenceConstructor<'a> {
    path: &'a Path,
    class: &'a str,
    code: JavaCode,
}

impl<'a> JavaPreferenceConstructor<'a> {
    pub(super) fn read(
        path: &'a Path,
        source: &str,
        class: &'a str,
    ) -> Result<Vec<String>, Failure> {
        let reader = Self {
            path,
            class,
            code: JavaCode::new(source),
        };
        let found = reader.constructors();
        let defaults = Self::defaults(&found);
        let preferences = Self::preferences(&found);
        let parameters = reader.parameters(&found, defaults, &preferences)?;
        reader.checked(&parameters)
    }

    fn parameters(
        &self,
        found: &[Vec<(String, String)>],
        defaults: usize,
        preferences: &[Vec<(String, String)>],
    ) -> Result<Vec<(String, String)>, Failure> {
        match (found, defaults, preferences) {
            ([], _, _) | (_, 1.., _) => Ok(Vec::new()),
            (_, 0, [parameters]) | ([parameters], 0, []) => Ok(parameters.clone()),
            (_, 0, []) => Err(self.failure("No Java browser constructor can be called without arguments or with only PreferenceStrings.")),
            _ => Err(self.failure("More than one Java PreferenceStrings constructor is available; browser injection needs an unambiguous constructor.")),
        }
    }

    fn defaults(found: &[Vec<(String, String)>]) -> usize {
        found
            .iter()
            .filter(|parameters| parameters.is_empty())
            .count()
    }

    fn preferences(found: &[Vec<(String, String)>]) -> Vec<Vec<(String, String)>> {
        found
            .iter()
            .filter(|parameters| Self::preference_only(parameters))
            .cloned()
            .collect()
    }

    fn preference_only(parameters: &[(String, String)]) -> bool {
        parameters
            .iter()
            .all(|(kind, _)| kind == "PreferenceStrings")
    }

    fn constructors(&self) -> Vec<Vec<(String, String)>> {
        self.code
            .declaration("class", self.class)
            .and_then(|declared| self.code.body_after(declared))
            .map(|body| self.code.member_headers(&body))
            .unwrap_or_default()
            .iter()
            .filter_map(|header| {
                JavaDeclaration::parse(&self.code.text(header)).constructor_parameters(self.class)
            })
            .collect()
    }

    fn checked(&self, parameters: &[(String, String)]) -> Result<Vec<String>, Failure> {
        let unsupported = parameters
            .iter()
            .find(|(kind, _)| kind != "PreferenceStrings");
        unsupported.map_or_else(
            || Ok(parameters.iter().map(|(_, name)| name.clone()).collect()),
            |(kind, _)| Err(self.failure(&format!("Java browser constructor injection supports PreferenceStrings; {kind} needs its own provider adapter."))),
        )
    }

    fn failure(&self, message: &str) -> Failure {
        let at = self
            .code
            .declaration("class", self.class)
            .unwrap_or_default();
        Failure::one(diagnostic(
            1405,
            message,
            None,
            source_span(&self.path.to_string_lossy(), at, at + 1),
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::JavaPreferenceConstructor;
    use std::path::Path;

    #[test]
    fn preference_constructors_preserve_order_and_ignore_other_class_constructors() {
        let source = "class Other { Other(String ignored) {} }\nclass ChoiceService {\n private final PreferenceStrings preferences;\n ChoiceService(final PreferenceStrings preferences) { this.preferences = preferences; }\n String answer() { return preferences.read(\"tachyon.platform\"); }\n}";
        let names = JavaPreferenceConstructor::read(
            Path::new("client/services/ChoiceService.java"),
            source,
            "ChoiceService",
        )
        .expect("typed browser constructor");
        assert_eq!(names, ["preferences"]);
    }

    #[test]
    fn an_existing_no_argument_constructor_keeps_its_factory_path_with_overloads() {
        for source in [
            "class ChoiceService { ChoiceService() {} ChoiceService(String unrelated) {} }",
            "class ChoiceService { ChoiceService(PreferenceStrings port) {} ChoiceService() {} }",
            "class ChoiceService { String answer() { return \"implicit\"; } }",
        ] {
            let names = JavaPreferenceConstructor::read(
                Path::new("client/services/ChoiceService.java"),
                source,
                "ChoiceService",
            )
            .expect("existing no-argument factory");
            assert!(names.is_empty(), "{source}");
        }
    }

    #[test]
    fn one_preference_constructor_is_selected_among_unrelated_argument_overloads() {
        let source = "class ChoiceService { ChoiceService(String unrelated) {} ChoiceService(PreferenceStrings port) {} }";
        let names = JavaPreferenceConstructor::read(
            Path::new("client/services/ChoiceService.java"),
            source,
            "ChoiceService",
        )
        .expect("unambiguous preference constructor");
        assert_eq!(names, ["port"]);
    }

    #[test]
    fn unsupported_and_ambiguous_argument_only_browser_constructors_are_located_errors() {
        for (source, message) in [
            (
                "class ChoiceService { ChoiceService(String unrelated) {} }",
                "String needs its own provider adapter",
            ),
            (
                "class ChoiceService { ChoiceService(String unrelated) {} ChoiceService(int other) {} }",
                "No Java browser constructor",
            ),
            (
                "class ChoiceService { ChoiceService(PreferenceStrings port) {} ChoiceService(PreferenceStrings first, PreferenceStrings second) {} }",
                "unambiguous constructor",
            ),
        ] {
            let failure = JavaPreferenceConstructor::read(
                Path::new("client/services/ChoiceService.java"),
                source,
                "ChoiceService",
            )
            .expect_err("unsupported constructor");
            let rendered = failure.to_string();
            assert!(rendered.contains("TY1405"), "{rendered}");
            assert!(rendered.contains("ChoiceService.java"), "{rendered}");
            assert!(rendered.contains(message), "{rendered}");
        }
    }
}
