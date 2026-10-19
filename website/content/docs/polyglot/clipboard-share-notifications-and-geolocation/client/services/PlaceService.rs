#[Service]
#[derive(Default)]
struct PlaceService {
    geolocation: Geolocation,
    clipboard: Clipboard,
    place: String,
    problem: String,
}

impl PlaceService {
    pub fn locate(&mut self) {
        let answer = self.geolocation.current_position();
        self.report(answer);
    }

    pub fn copy(&mut self) {
        let answer = self.clipboard.write_text(&self.place);
        self.report(answer);
    }

    fn report(&mut self, answer: Result<(), String>) {
        if let Err(message) = answer {
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
