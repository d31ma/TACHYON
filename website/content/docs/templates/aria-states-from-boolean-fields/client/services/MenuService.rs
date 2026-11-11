#[Service]
struct MenuService {
    expanded: bool,
}

impl MenuService {
    fn new() -> Self {
        Self { expanded: false }
    }

    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }
}
