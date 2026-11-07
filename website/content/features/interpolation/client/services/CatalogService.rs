use crate::models::Product;

#[Service]
pub struct CatalogService {
    pub title: String,
    pub count: i32,
    pub product: Product,
}

impl CatalogService {
    pub fn new() -> Self {
        Self {
            title: String::from("Catalog"),
            count: 6,
            product: Product {
                name: String::from("Anvil"),
                price: String::from("£42.00"),
            },
        }
    }
}
