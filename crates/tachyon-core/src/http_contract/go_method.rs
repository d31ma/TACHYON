use std::ops::Range;

pub(crate) struct GoMethod {
    pub(crate) markers: Vec<String>,
    pub(crate) signature: String,
    pub(crate) has_body: bool,
    pub(crate) whole: Range<usize>,
    pub(crate) starts_context: bool,
}

impl GoMethod {
    pub(crate) fn declaration(&self, client: &str) -> Result<String, String> {
        let name = self.signature.split('(').next().unwrap_or("");
        (!self.has_body).then(|| self.lines()).ok_or_else(|| {
            format!(
                "{client}.{name} has a body. An HTTPClient has no implementation: move it into a service, then run the fix again."
            )
        })
    }

    fn lines(&self) -> String {
        self.markers
            .iter()
            .map(|marker| format!("\t{marker}\n"))
            .chain(
                self.signature
                    .lines()
                    .map(|line| format!("\t{}\n", line.trim_start_matches('\t'))),
            )
            .collect()
    }
}
