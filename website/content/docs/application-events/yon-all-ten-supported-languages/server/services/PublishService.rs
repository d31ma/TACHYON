#[Service]
struct PublishService {
  #[Needs]
  application_event_publisher: std::sync::Arc<ApplicationEventPublisher>,
}
impl PublishService {
  #[OnStart]
  fn initialize(&self) {
    self.application_event_publisher.publish_event(crate::models::ProbeReady { value: 7 }, false);
  }
}
