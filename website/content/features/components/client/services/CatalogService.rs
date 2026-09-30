#[Service]
pub struct CatalogService {
    pub products: TacValue,
}

impl CatalogService {
    pub fn new() -> Self {
        Self {
            products: TacValue::List(vec![
                Self::product("anvil", "Anvil", "£42.00"),
                Self::product("rope", "Rope", "£8.00"),
            ]),
        }
    }

    fn product(id: &str, name: &str, price: &str) -> TacValue {
        TacValue::Object(vec![
            (String::from("id"), TacValue::Text(String::from(id))),
            (String::from("name"), TacValue::Text(String::from(name))),
            (String::from("price"), TacValue::Text(String::from(price))),
        ])
    }
}
