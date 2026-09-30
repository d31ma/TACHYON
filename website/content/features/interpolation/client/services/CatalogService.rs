#[Service]
pub struct CatalogService {
    pub title: String,
    pub count: i32,
    pub product: TacValue,
}

impl CatalogService {
    pub fn new() -> Self {
        Self {
            title: String::from("Catalog"),
            count: 6,
            product: TacValue::Object(vec![
                (String::from("name"), TacValue::Text(String::from("Anvil"))),
                (String::from("price"), TacValue::Text(String::from("£42.00"))),
            ]),
        }
    }
}
