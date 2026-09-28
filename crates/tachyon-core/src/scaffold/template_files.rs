pub(crate) struct TemplateFiles;

impl TemplateFiles {
    const ALL: &'static [(&'static str, &'static str)] =
        include!(concat!(env!("OUT_DIR"), "/scaffold_templates.rs"));

    pub(crate) fn under(folder: &str, output: &str) -> Vec<(String, String)> {
        let prefix = format!("{folder}/");
        Self::ALL
            .iter()
            .filter_map(|(path, text)| Self::renamed(path, text, &prefix, output))
            .collect()
    }

    fn renamed(path: &str, text: &str, prefix: &str, output: &str) -> Option<(String, String)> {
        let rest = path.strip_prefix(prefix)?;
        Some((format!("{output}{rest}"), String::from(text)))
    }
}
