use crate::models::Guide;

#[RequestRoute("/guide")]
#[Public]
struct GuideController {}
impl GuideController {
    fn new() -> Self {
        Self {}
    }

    #[GetRoute]
    fn get_guide(&self) -> Guide {
        Guide { title: String::from("Models hold types"), pages: 3 }
    }
}
