use super::contract_role::ContractRole;

pub(crate) struct FsharpClassType {
    pub(crate) role: ContractRole,
    pub(crate) marker: String,
    pub(crate) name: String,
    pub(crate) header: usize,
    pub(crate) body: Vec<usize>,
}
