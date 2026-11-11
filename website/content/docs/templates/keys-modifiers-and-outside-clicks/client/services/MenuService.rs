#[Service]
#[derive(Default)]
struct MenuService {
    expanded: bool,
}

impl MenuService {
    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    pub fn close(&mut self) {
        self.expanded = false;
    }
}
