#[Service]
struct CounterService {
    count: i64,
    #[SessionState]
    visits: i64,
    #[LocalState]
    theme: String,
}

impl CounterService {
    fn new() -> Self {
        Self { count: 0, visits: 0, theme: String::from("light") }
    }

    fn increment(&mut self) {
        self.count += 1;
    }
}
