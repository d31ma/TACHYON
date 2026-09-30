#[Service]
#[derive(Default)]
struct BasketService {
    count: i64,
}

impl BasketService {
    fn doubled(&self) -> i64 {
        self.count * 2
    }

    #[OnStart]
    fn initialize(&mut self) { self.count = 6; }
}
