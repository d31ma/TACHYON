use std::path::Path;

pub(super) struct NativeCapabilities {
    source: String,
    excluded: Vec<String>,
}

impl NativeCapabilities {
    const NATIVE_PROVIDERS: [(&'static str, &'static str); 2] = [
        ("FilePicker", "NativeCapabilities.filePicker()"),
        ("WindowControls", "NativeCapabilities.windowControls()"),
    ];
    const PAGE_PROVIDERS: [(&'static str, &'static str); 1] = [("FilePicker", "FilePicker()")];

    pub(super) fn swift_service(source: &str, path: &Path, class: &str) -> (String, Vec<String>) {
        Self::rewritten(source, path, class, &Self::NATIVE_PROVIDERS)
    }

    pub(super) fn swift_page_service(
        source: &str,
        path: &Path,
        class: &str,
    ) -> (String, Vec<String>) {
        Self::rewritten(source, path, class, &Self::PAGE_PROVIDERS)
    }

    fn rewritten(
        source: &str,
        path: &Path,
        class: &str,
        providers: &[(&str, &'static str)],
    ) -> (String, Vec<String>) {
        let fields = Self::fields(source, path, class, providers);
        let compilation = fields.into_iter().fold(
            Self {
                source: source.to_owned(),
                excluded: Vec::new(),
            },
            |mut compilation, (name, kind, provider)| {
                compilation.rewrite_field(&name, &kind, provider);
                compilation
            },
        );
        (compilation.source, compilation.excluded)
    }

    fn fields(
        source: &str,
        path: &Path,
        class: &str,
        providers: &[(&str, &'static str)],
    ) -> Vec<(String, String, &'static str)> {
        crate::template::required_fields::RequiredFields::constructed(path, source, class)
            .into_iter()
            .filter_map(|(name, kind)| {
                Self::provider(&kind, providers).map(|provider| (name, kind, provider))
            })
            .collect()
    }

    fn provider(kind: &str, providers: &[(&str, &'static str)]) -> Option<&'static str> {
        providers
            .iter()
            .find(|(known, _)| *known == kind)
            .map(|(_, provider)| *provider)
    }

    fn rewrite_field(&mut self, name: &str, kind: &str, provider: &str) {
        let excluded = &mut self.excluded;
        self.source = self
            .source
            .split_inclusive('\n')
            .map(|line| Self::rewrite_line(line, name, kind, provider, excluded))
            .collect();
    }

    fn rewrite_line(
        line: &str,
        name: &str,
        kind: &str,
        provider: &str,
        excluded: &mut Vec<String>,
    ) -> String {
        let (prefix, declared) = line.split_once(':').unwrap_or((line, ""));
        if prefix.split_whitespace().last() == Some(name) && declared.trim() == kind {
            excluded.push(name.to_owned());
            return format!("{prefix}: {kind} = {provider}\n");
        }
        line.to_owned()
    }
}
