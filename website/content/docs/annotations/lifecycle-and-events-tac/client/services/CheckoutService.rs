use crate::models::PlacedOrder;

#[derive(Default)]
struct CheckoutService {
    placed: i64,
    application_event_publisher: ApplicationEventPublisher,
}
impl CheckoutService {
    #[OnStart]
    fn start(&mut self) { self.place(); }

    fn place(&mut self) {
        self.placed += 1;
        let order = PlacedOrder { status: String::from("paid"), total: 25 };
        self.application_event_publisher.publish_event(order, true);
    }

    #[OnStop]
    fn stop(&mut self) { self.placed = 0; }
}
