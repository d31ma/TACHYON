mod client_declaration;
mod concrete_client;
mod context_members;
mod contract_check;
mod contract_form;
mod contract_member;
mod contract_rewrite;
mod indented_members;
mod member_scanner;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

use crate::Failure;
use concrete_client::ConcreteClient;
use contract_check::ContractCheck;
use contract_rewrite::ContractRewrite;
pub(crate) use contract_rewrite::SourceEdit;
use std::path::Path;

pub(crate) struct HttpContract;

impl HttpContract {
    pub(crate) fn check(path: &Path, source: &str) -> Result<(), Failure> {
        ContractCheck::check(path, source)
    }

    pub(crate) fn rewrite(path: &Path, source: &str) -> Result<Vec<SourceEdit>, String> {
        ContractRewrite::edits(path, source)
    }

    pub(crate) fn concrete(path: &Path, source: &str, environment: &str) -> Option<String> {
        ConcreteClient::lower(path, source, environment)
    }
}
