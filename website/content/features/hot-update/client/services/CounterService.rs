#[Service]
#[derive(Default)]
struct CounterService {
    count: i64,
}

impl CounterService {
    fn increment(&mut self) {
        self.count += 1;
    }
}
