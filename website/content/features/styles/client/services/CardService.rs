#[Service]
pub struct CardService {
    pub name: &'static str,
}

impl CardService {
    pub fn new() -> Self {
        Self { name: "Anvil" }
    }
}
