mod concrete_contract;
mod context_members;
mod context_template;
mod contract_check;
mod contract_declaration;
mod contract_form;
mod contract_member;
mod contract_rewrite;
mod contract_role;
mod fsharp_class_type;
mod fsharp_contract;
mod fsharp_contract_rewrite;
mod fsharp_declaration;
mod go_contract_rewrite;
mod go_method;
mod indented_members;
mod indented_syntax;
mod marker_line;
mod member_scanner;
mod rust_contract_rewrite;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod store_tests;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

use crate::Failure;
use concrete_contract::ConcreteContract;
use contract_check::ContractCheck;
use contract_rewrite::ContractRewrite;
pub(crate) use contract_rewrite::SourceEdit;
use fsharp_contract::FsharpContract;
use fsharp_contract_rewrite::FsharpContractRewrite;
use std::path::Path;

pub(crate) struct DeclaredContract;

impl DeclaredContract {
    pub(crate) fn check(path: &Path, source: &str) -> Result<(), Failure> {
        ContractCheck::check(path, source)
    }

    pub(crate) fn rewrite(path: &Path, source: &str) -> Result<Vec<SourceEdit>, String> {
        FsharpContractRewrite::edits(path, source)
            .unwrap_or_else(|| ContractRewrite::edits(path, source))
    }

    pub(crate) fn concrete(path: &Path, source: &str, environment: &str) -> Option<String> {
        FsharpContract::lower(path, source, environment)
            .or_else(|| ConcreteContract::lower(path, source, environment))
    }
}
