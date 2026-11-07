#[Service]
struct CheckoutService {
  #[Needs]
  application_event_publisher: std::sync::Arc<ApplicationEventPublisher>,
}
impl CheckoutService {
  #[OnStart]
  fn start(&self) {
    let order = crate::models::PlacedOrder { status: String::from("paid"), total: 25 };
    self.application_event_publisher.publish_event(order, false);
  }

  #[OnStop]
  fn stop(&self) {}
}
