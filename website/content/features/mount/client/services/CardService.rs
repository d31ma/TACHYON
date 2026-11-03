#[Service]
pub struct CardService {
    pub name: &'static str,
    pub status: &'static str,
}

impl CardService {
    pub fn new() -> Self {
        Self { name: "Anvil", status: "waiting" }
    }

    #[OnStart]
    pub fn activate(&mut self) {
        self.status = "active";
    }
}
