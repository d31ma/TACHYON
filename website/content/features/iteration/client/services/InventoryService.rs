#[Service]
pub struct InventoryService {
    pub products: TacValue,
}

impl InventoryService {
    const NAME: &'static str = "name";

    pub fn new() -> Self {
        Self {
            products: TacValue::List(vec![
                TacValue::Object(vec![(String::from(Self::NAME), TacValue::Text(String::from("Anvil")))]),
                TacValue::Object(vec![(String::from(Self::NAME), TacValue::Text(String::from("Rope")))]),
            ]),
        }
    }
}
