#[Store]
struct OrderStore;

impl OrderStore {
    fn all() -> Vec<&'static str> {
        vec!["anvil"]
    }
}

impl OrderStore {
    fn new() -> Self { Self }
}
