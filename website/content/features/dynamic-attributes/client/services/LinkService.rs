#[Service]
pub struct LinkService {
    pub topic: TacValue,
    pub state: String,
    pub message: String,
}

impl LinkService {
    pub fn new() -> Self {
        Self {
            topic: TacValue::Object(vec![
                (String::from("slug"), TacValue::Text(String::from("routing"))),
                (String::from("title"), TacValue::Text(String::from("Routing"))),
            ]),
            state: String::from("ready"),
            message: String::from("Everything is up to date."),
        }
    }
}
