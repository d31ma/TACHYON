#[Service]
#[derive(Default)]
struct PlaceService {
    geolocation: Geolocation,
    clipboard: Clipboard,
    place: String,
    problem: String,
}

impl PlaceService {
    fn locate(&mut self) {
        if let Err(message) = self.geolocation.current_position() {
            self.problem = message;
        }
    }

    fn copy(&mut self) {
        if let Err(message) = self.clipboard.write_text(&self.place) {
            self.problem = message;
        }
    }

    #[OnEvent]
    fn found(&mut self, found: PositionFound) {
        self.place = format!("{}, {}", found.latitude, found.longitude);
    }

    #[OnEvent]
    fn failed(&mut self, failed: BrowserCallFailed) {
        self.problem = failed.message;
    }
}
