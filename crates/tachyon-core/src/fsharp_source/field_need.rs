pub(crate) struct FieldNeed {
    pub(crate) owner: usize,
    pub(crate) markers: Vec<usize>,
    pub(crate) name: String,
    pub(crate) provider: String,
}

impl FieldNeed {
    const MEMBER: &'static str = "member val ";
    const SETTABLE: &'static str = "with get, set";

    pub(crate) fn read(
        trimmed: &str,
        owner: usize,
        markers: Vec<usize>,
        named: Option<String>,
    ) -> Option<Self> {
        let declaration = trimmed.strip_prefix(Self::MEMBER)?;
        declaration.strip_suffix(Self::SETTABLE)?;
        let (typed, _) = declaration.split_once('=')?;
        let (name, kind) = typed.split_once(':')?;
        let kind = kind.trim();
        let name = name.trim();
        let written = [name, kind].iter().all(|part| !part.is_empty());
        written.then(|| Self {
            owner,
            markers,
            name: name.to_owned(),
            provider: named.unwrap_or_else(|| kind.to_owned()),
        })
    }

    pub(crate) fn parameter(&self) -> String {
        let mut letters = self.name.chars();
        let first = letters.next().map(|letter| letter.to_ascii_lowercase());
        let name: String = first.into_iter().chain(letters).collect();
        format!("{name}: {}", self.provider)
    }
}

#[cfg(test)]
mod tests {
    use super::FieldNeed;

    #[test]
    fn a_settable_typed_property_is_a_field_need() {
        let line =
            "member val OrderStore : OrderStore = Unchecked.defaultof<OrderStore> with get, set";
        let need = FieldNeed::read(line, 3, vec![5], None).unwrap_or_else(|| panic!("need"));
        assert_eq!(need.name, "OrderStore");
        assert_eq!(need.provider, "OrderStore");
        assert_eq!(need.parameter(), "orderStore: OrderStore");
        let named = FieldNeed::read(line, 3, vec![5], Some(String::from("FastStore")));
        assert!(named.is_some_and(|need| need.provider == "FastStore"));
    }

    #[test]
    fn a_read_only_or_untyped_property_is_not() {
        let read_only =
            "member val OrderStore : OrderStore = Unchecked.defaultof<OrderStore> with get";
        assert!(FieldNeed::read(read_only, 0, Vec::new(), None).is_none());
        let untyped = "member val OrderStore = OrderStore() with get, set";
        assert!(FieldNeed::read(untyped, 0, Vec::new(), None).is_none());
    }
}
