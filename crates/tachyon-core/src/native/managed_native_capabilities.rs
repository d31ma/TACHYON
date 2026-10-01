use crate::project::NativeCompanion;
use std::fmt::Write;
use std::path::Path;

pub(super) struct ManagedNativeCapabilities;

impl ManagedNativeCapabilities {
    #[cfg(test)]
    pub(super) const KOTLIN: &'static str = concat!(
        include_str!("capabilities/FilePickerStatus.kt"),
        include_str!("capabilities/FilePickerResult.kt"),
        include_str!("capabilities/FilePickerOptions.kt"),
        include_str!("capabilities/NativeCapabilityOwner.kt"),
        include_str!("capabilities/NativeCapabilities.kt"),
        include_str!("capabilities/FilePicker.kt")
    );
    #[cfg(test)]
    pub(super) const KOTLIN_WINDOWS: &'static str = include_str!("capabilities/WindowControls.kt");

    pub(super) fn service(
        language: NativeCompanion,
        path: &Path,
        source: &str,
        class: &str,
    ) -> (String, Vec<String>) {
        let fields = Self::fields(language, path, source, class);
        Self::rewrite(language, source, fields)
    }

    fn fields(
        language: NativeCompanion,
        path: &Path,
        source: &str,
        class: &str,
    ) -> Vec<(String, String)> {
        let fields = Self::constructed(language, path, source, class);
        fields
            .into_iter()
            .filter(|(_, declared)| Self::kind(declared).is_some())
            .collect()
    }

    fn constructed(
        language: NativeCompanion,
        path: &Path,
        source: &str,
        class: &str,
    ) -> Vec<(String, String)> {
        if ![NativeCompanion::CSharp, NativeCompanion::Kotlin].contains(&language) {
            return Vec::new();
        }
        crate::template::required_fields::RequiredFields::constructed(path, source, class)
    }

    fn kind(declared: &str) -> Option<&str> {
        let kind = declared.split_whitespace().last().unwrap_or_default();
        ["FilePicker", "WindowControls"]
            .contains(&kind)
            .then_some(kind)
    }

    fn rewrite(
        language: NativeCompanion,
        source: &str,
        fields: Vec<(String, String)>,
    ) -> (String, Vec<String>) {
        let mut output = source.to_owned();
        let mut excluded = Vec::new();
        for (name, declared) in fields {
            Self::apply(language, &mut output, &mut excluded, name, &declared);
        }
        (output, excluded)
    }

    fn apply(
        language: NativeCompanion,
        output: &mut String,
        excluded: &mut Vec<String>,
        name: String,
        declared: &str,
    ) {
        let kind = Self::kind(declared).unwrap_or_default();
        let (rewritten, changed) = Self::field(language, output, &name, kind);
        if changed {
            *output = rewritten;
            excluded.push(name);
        }
    }

    fn field(language: NativeCompanion, source: &str, name: &str, kind: &str) -> (String, bool) {
        let mut changed = false;
        let mut output = String::new();
        for line in source.split_inclusive('\n') {
            let found = Self::matches_line(language, line, name, kind);
            Self::line(language, line, kind, found, &mut output, &mut changed);
        }
        (output, changed)
    }

    fn line(
        language: NativeCompanion,
        line: &str,
        kind: &str,
        found: bool,
        output: &mut String,
        changed: &mut bool,
    ) {
        if !found {
            output.push_str(line);
            return;
        }
        Self::replacement(language, line, kind, output);
        *changed = true;
    }

    fn replacement(language: NativeCompanion, line: &str, kind: &str, output: &mut String) {
        Self::remove_marker(output);
        let line = line.trim_end().trim_end_matches(';');
        let line = line.replace("required ", "").replace("lateinit ", "");
        let provider = Self::provider(language, kind);
        let _ = writeln!(output, "{line} = {provider}");
    }

    fn matches_line(language: NativeCompanion, line: &str, name: &str, kind: &str) -> bool {
        match language {
            NativeCompanion::CSharp => Self::csharp_line(line, name, kind),
            NativeCompanion::Kotlin => Self::kotlin_line(line, name, kind),
            _ => false,
        }
    }

    fn csharp_line(line: &str, name: &str, kind: &str) -> bool {
        line.split([';', '{']).next().is_some_and(|declaration| {
            let words = declaration.split_whitespace().collect::<Vec<_>>();
            [words.last().copied() == Some(name), words.contains(&kind)] == [true, true]
        })
    }

    fn kotlin_line(line: &str, name: &str, kind: &str) -> bool {
        line.split_once(':').is_some_and(|(prefix, declared)| {
            [
                prefix.split_whitespace().last() == Some(name),
                declared.trim() == kind,
            ] == [true, true]
        })
    }

    fn remove_marker(output: &mut String) {
        let marker_start = output
            .trim_end_matches('\n')
            .rfind('\n')
            .map_or(0, |at| at + 1);
        let markers = ["@Needs", "@Needs()", "[Needs]", "[Needs()]"];
        if markers.contains(&output[marker_start..].trim()) {
            output.truncate(marker_start);
        }
    }

    fn provider(language: NativeCompanion, kind: &str) -> String {
        if language == NativeCompanion::CSharp {
            return format!("new {kind}(NativeCapabilities.Provider());");
        }
        format!("{kind}(NativeCapabilities.provider())")
    }
}
