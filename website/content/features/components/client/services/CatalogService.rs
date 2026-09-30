use crate::models::Product;

#[Service]
pub struct CatalogService {
    pub products: Vec<Product>,
}

impl CatalogService {
    pub fn new() -> Self {
        Self {
            products: vec![
                Product { id: String::from("anvil"), name: String::from("Anvil"), price: String::from("£42.00") },
                Product { id: String::from("rope"), name: String::from("Rope"), price: String::from("£8.00") },
            ],
        }
    }
}
