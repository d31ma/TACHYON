#[Service]
#[derive(Default)]
struct StatusService {
    page_visibility: PageVisibility,
    online: bool,
    visible: bool,
}

impl StatusService {
    #[OnEvent({"replay":true})]
    fn connectivity_changed(&mut self, changed: ConnectivityChanged) {
        self.online = changed.online;
    }

    fn check(&mut self) {
        self.visible = self.page_visibility.visible().unwrap_or(true);
    }
}
