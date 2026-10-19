use crate::models::ReleaseChanged;

#[Service]
#[derive(Default)]
struct ReleaseService {
    release: String,
    application_event_publisher: ApplicationEventPublisher,
}

impl ReleaseService {
    pub fn announce(&mut self) {
        let changed = ReleaseChanged { release: String::from("TACHYON") };
        self.application_event_publisher.publish_event(changed, true);
    }

    #[OnEvent({"replay":true})]
    fn receive(&mut self, changed: ReleaseChanged) { self.release = changed.release; }
}
