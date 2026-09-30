use crate::models::Tab;

#[Service]
pub struct BasketService {
    pub count: i32,
    pub label: String,
    selected: String,
    pub tabs: Vec<Tab>,
}

impl BasketService {
    pub fn new() -> Self {
        Self {
            count: 0,
            label: String::new(),
            selected: String::new(),
            tabs: vec![Tab {
                id: String::from("a"),
                name: String::from("A"),
            }],
        }
    }

    pub fn increment(&mut self) {
        self.count += 1;
    }

    pub fn select(&mut self, id: String) {
        self.selected = id;
    }
}
