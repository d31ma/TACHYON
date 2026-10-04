#[RequestRoute("/events")]
#[Public]
struct EventsController {
  #[Needs]
  event_service: std::sync::Arc<EventService>,
}
impl EventsController {
  #[GetRoute]
  fn get_events(&self) -> Result<Json, YonResourceError> { self.event_service.report() }
}
