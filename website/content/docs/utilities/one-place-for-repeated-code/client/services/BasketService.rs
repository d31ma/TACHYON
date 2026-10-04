#[Service]
struct BasketService {
    total: u32,
}

impl Default for BasketService {
    fn default() -> Self {
        Self { total: 1250 }
    }
}

impl BasketService {
    fn total_label(&self) -> String {
        PriceUtils::label(self.total)
    }
}
