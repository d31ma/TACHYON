use crate::models::Guide;

struct GuideService {
    title: String,
    pages: i64,
}

impl GuideService {
    fn new() -> Self {
        let guide = Guide { title: String::from("Models hold types"), pages: 3 };
        Self { title: guide.title, pages: guide.pages }
    }
}
