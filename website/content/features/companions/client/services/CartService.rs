use crate::models::CartTotal;

#[Service]
#[derive(Default)]
struct CartService {
    total: i64,
    application_event_publisher: ApplicationEventPublisher,
}

impl CartService {
    pub fn add(&mut self) {
        self.total += 1;
        self.application_event_publisher.publish_event(CartTotal { total: self.total }, true);
    }
}
