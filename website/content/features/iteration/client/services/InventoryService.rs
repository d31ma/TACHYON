use crate::models::Product;

#[Service]
pub struct InventoryService {
    pub products: Vec<Product>,
}

impl InventoryService {
    pub fn new() -> Self {
        Self {
            products: vec![Product { name: String::from("Anvil") }, Product { name: String::from("Rope") }],
        }
    }
}
