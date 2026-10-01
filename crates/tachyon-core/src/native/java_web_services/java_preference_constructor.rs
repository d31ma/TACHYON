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
        let parameters = reader.parameters()?;
        reader.checked(&parameters)
    }

    fn parameters(&self) -> Result<Vec<(String, String)>, Failure> {
        let found = self.constructors();
        match found.as_slice() {
            [] => Ok(Vec::new()),
            [parameters] => Ok(parameters.clone()),
            _ => Err(self.failure("A Java browser service has one constructor.")),
        }
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
    fn unsupported_and_overloaded_browser_constructors_are_located_errors() {
        for (source, message) in [
            (
                "class ChoiceService { ChoiceService(String unrelated) {} }",
                "String needs its own provider adapter",
            ),
            (
                "class ChoiceService { ChoiceService() {} ChoiceService(PreferenceStrings port) {} }",
                "one constructor",
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
