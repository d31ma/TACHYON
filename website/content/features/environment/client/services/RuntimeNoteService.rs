#[Service]
struct RuntimeNoteService {
    runtime: String,
}

impl RuntimeNoteService {
    fn new() -> Self {
        Self { runtime: String::from("Rust, compiled by rustc") }
    }

    pub fn shows_notes(&self, development: bool) -> bool {
        development
    }
}
