#[Service]
#[derive(Default)]
struct TallyService {
    count: i64,
}

impl TallyService {
    pub fn add(&mut self, amount: i64) {
        self.count += amount;
    }

    pub fn summary(&self, unit: &str) -> String {
        format!("{} {unit}", self.count)
    }
}
