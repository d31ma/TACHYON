#[Service]
pub struct StockService {
    pub products: TacValue,
    pub restocking: bool,
}

impl StockService {
    pub fn new() -> Self {
        Self { products: TacValue::List(Vec::new()), restocking: true }
    }
}
