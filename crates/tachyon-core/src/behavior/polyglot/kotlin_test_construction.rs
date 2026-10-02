use super::super::testing::Unit;
use super::typed_test_construction::TypedTestConstruction;
use super::{Module, Type};

#[cfg(test)]
mod tests;

pub(super) struct KotlinTestConstruction;

impl KotlinTestConstruction {
    pub(super) fn members(module: &Module, owner: &Type, unit: &Unit) -> String {
        TypedTestConstruction::members(module, owner, unit, "kt")
    }
}
