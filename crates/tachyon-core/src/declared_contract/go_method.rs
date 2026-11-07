use super::contract_declaration::ContractDeclaration;
use std::ops::Range;

pub(crate) struct GoMethod {
    pub(crate) markers: Vec<String>,
    pub(crate) signature: String,
    pub(crate) has_body: bool,
    pub(crate) whole: Range<usize>,
    pub(crate) starts_context: bool,
}

impl GoMethod {
    pub(crate) fn declaration(&self, owner: &ContractDeclaration) -> Result<String, String> {
        let name = self.signature.split('(').next().unwrap_or("");
        (!self.has_body).then(|| self.lines()).ok_or_else(|| {
            format!(
                "{}.{name} has a body. {} Then run the fix again.",
                owner.name, owner.role.no_implementation
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
