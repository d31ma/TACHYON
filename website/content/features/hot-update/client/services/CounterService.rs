#[Service]
#[derive(Default)]
struct CounterService {
    count: i64,
}

impl CounterService {
    pub fn increment(&mut self) {
        self.count += 1;
    }
}
