#[Service]
pub struct BasketService {
    pub count: i32,
    pub label: String,
    selected: String,
    pub tabs: TacValue,
}

impl BasketService {
    pub fn new() -> Self {
        Self {
            count: 0,
            label: String::new(),
            selected: String::new(),
            tabs: TacValue::List(vec![TacValue::Object(vec![
                (String::from("id"), TacValue::Text(String::from("a"))),
                (String::from("name"), TacValue::Text(String::from("A"))),
            ])]),
        }
    }

    pub fn increment(&mut self) {
        self.count += 1;
    }

    pub fn select(&mut self, id: String) {
        self.selected = id;
    }
}
