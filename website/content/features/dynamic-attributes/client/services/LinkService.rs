use crate::models::Topic;

#[Service]
pub struct LinkService {
    pub topic: Topic,
    pub state: String,
    pub message: String,
}

impl LinkService {
    pub fn new() -> Self {
        Self {
            topic: Topic {
                slug: String::from("routing"),
                title: String::from("Routing"),
            },
            state: String::from("ready"),
            message: String::from("Everything is up to date."),
        }
    }
}
