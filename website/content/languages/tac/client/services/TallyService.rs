#[Service]
pub struct TallyService {
    pub count: i32,
    pub runtime: &'static str,
}

impl TallyService {
    pub fn new() -> Self {
        Self { count: 6, runtime: "Rust, compiled by rustc" }
    }

    pub fn doubled(&self) -> i32 {
        self.count * 2
    }
}