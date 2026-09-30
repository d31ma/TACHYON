#[Service]
pub struct CounterService {
    pub count: i32,
}

impl CounterService {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    pub fn increment(&mut self) {
        self.count += 1;
    }
}
