use std::path::{Path, PathBuf};

pub(super) struct ModelFile {
    pub(super) inside: PathBuf,
    pub(super) portable: String,
    pub(super) source: String,
    pub(super) plain: String,
    pub(super) schema: bool,
}

impl ModelFile {
    pub(super) fn go_source(&self) -> String {
        self.go_program()
    }

    fn go_program(&self) -> String {
        let position = format!("//line {}:1\n", self.portable);
        let code = crate::annotation_layout::masked_source(Path::new(&self.portable), &self.source)
            .unwrap_or_default();
        if code.split_whitespace().next() == Some("package") {
            return format!("{position}{}", self.plain);
        }
        format!("{position}package main; {}", self.plain)
    }
    pub(super) fn plain_source(&self) -> String {
        self.plain.clone()
    }

    pub(super) fn rust_yon_source(&self) -> String {
        let codecs = crate::route_types::Converters::rust_model_text(&self.plain);
        format!("use crate::{{Json, YonRead}};\n{}\n{codecs}", self.plain)
    }
}
