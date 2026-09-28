pub struct BasketService {
    count: i32,
    label: String,
}

impl BasketService {
    pub fn new() -> Self {
        Self { count: 6, label: String::from("Rust") }
    }

    fn doubled(&self) -> i32 {
        self.count * 2
    }
}