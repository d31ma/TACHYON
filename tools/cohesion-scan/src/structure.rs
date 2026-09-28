use quote::ToTokens;
use serde::Serialize;
use syn::{Attribute, File, Item, ItemImpl};

#[derive(Clone, Debug, Default, Serialize)]
pub struct Structure {
    pub types: Vec<String>,
    pub free_functions: Vec<String>,
    pub foreign_impls: Vec<String>,
    pub file_values: Vec<String>,
}

impl Structure {
    const TEST_MARKS: [&'static str; 3] = ["test", "tokio::test", "cfg(test)"];
    const ENTRY_POINT: &'static str = "main";

    #[must_use]
    pub fn of(file: &File) -> Self {
        let mut structure = Self::default();
        file.items
            .iter()
            .filter(|item| Self::is_counted(item))
            .for_each(|item| structure.add(item));
        structure.forgive_entry_point();
        structure.keep_foreign_impls();
        structure
    }

    #[must_use]
    pub fn total(&self) -> usize {
        self.types.len().saturating_sub(1)
            + self.free_functions.len()
            + self.foreign_impls.len()
            + self.file_values.len()
    }

    pub(crate) fn is_counted(item: &Item) -> bool {
        !Self::attributes(item).iter().any(Self::is_test_attribute)
    }

    fn is_test_attribute(attribute: &Attribute) -> bool {
        let text = attribute
            .meta
            .to_token_stream()
            .to_string()
            .replace(' ', "");
        Self::TEST_MARKS.contains(&text.as_str())
    }

    fn attributes(item: &Item) -> &[Attribute] {
        match item {
            Item::Struct(node) => &node.attrs,
            Item::Enum(node) => &node.attrs,
            Item::Trait(node) => &node.attrs,
            Item::Union(node) => &node.attrs,
            Item::Fn(node) => &node.attrs,
            Item::Impl(node) => &node.attrs,
            Item::Const(node) => &node.attrs,
            Item::Static(node) => &node.attrs,
            Item::Mod(node) => &node.attrs,
            _ => &[],
        }
    }

    fn add(&mut self, item: &Item) {
        match item {
            Item::Struct(node) => self.types.push(node.ident.to_string()),
            Item::Enum(node) => self.types.push(node.ident.to_string()),
            Item::Trait(node) => self.types.push(node.ident.to_string()),
            Item::Union(node) => self.types.push(node.ident.to_string()),
            Item::Fn(node) => self.free_functions.push(node.sig.ident.to_string()),
            Item::Impl(node) => self.foreign_impls.push(Self::implemented_type(node)),
            Item::Const(node) => self.file_values.push(node.ident.to_string()),
            Item::Static(node) => self.file_values.push(node.ident.to_string()),
            _ => {}
        }
    }

    fn implemented_type(node: &ItemImpl) -> String {
        let text = node.self_ty.to_token_stream().to_string();
        let without_generics = text.split('<').next().unwrap_or_default();
        let name = without_generics.rsplit("::").next().unwrap_or_default();
        name.trim().to_string()
    }

    fn forgive_entry_point(&mut self) {
        self.free_functions.retain(|name| name != Self::ENTRY_POINT);
    }

    fn keep_foreign_impls(&mut self) {
        let types = self.types.clone();
        self.foreign_impls.retain(|name| !types.contains(name));
    }
}
