use crate::models::ProbeReady;

#[RequestRoute("/probe")]
#[Public]
struct ProbeController {
  #[Needs]
  probe: std::sync::Arc<ProbeService>,
  #[Needs]
  publish: std::sync::Arc<PublishService>,
}
impl ProbeController {
  #[GetRoute]
  fn get_probe(&self) -> ProbeReady {
    ProbeReady { value: *self.probe.last.lock().unwrap() }
  }
}
