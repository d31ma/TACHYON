use crate::models::CountChanged;

#[Service]
#[derive(Default)]
struct CounterService {
    count: i64,
    application_event_publisher: ApplicationEventPublisher,
}

impl CounterService {
    fn add(&mut self) {
        self.count += 1;
        self.application_event_publisher.publish_event(CountChanged { count: self.count }, false);
    }
}
