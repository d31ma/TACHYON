use crate::models::Product;

#[Service]
pub struct StockService {
    pub products: Vec<Product>,
    pub restocking: bool,
}

impl StockService {
    pub fn new() -> Self {
        Self { products: Vec::new(), restocking: true }
    }
}
