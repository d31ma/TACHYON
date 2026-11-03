#[Service]
pub struct BadgeService {
    pub status: String,
}

impl BadgeService {
    pub fn new() -> Self {
        Self { status: String::from("ready") }
    }
}
