#[Service]
#[derive(Default)]
struct ActivityService {
    state: String,
}

impl ActivityService {
    #[OnEvent({"replay":true})]
    fn application_state_changed(&mut self, changed: ApplicationStateChanged) {
        self.state = changed.state;
    }
}
