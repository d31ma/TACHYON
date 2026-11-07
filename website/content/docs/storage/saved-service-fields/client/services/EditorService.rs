#[Service]
struct EditorService {
    #[SessionState]
    draft: String,
    #[LocalState]
    theme: String,
}

impl EditorService {
    fn new() -> Self {
        Self { draft: String::new(), theme: String::from("calm") }
    }
}
