#[Service]
struct ProbeService { last: std::sync::Mutex<i64> }
impl ProbeService {
  fn new() -> Self { Self { last: std::sync::Mutex::new(0) } }

  #[OnEvent]
  fn observe(&self, ready: crate::models::ProbeReady) { *self.last.lock().unwrap() = ready.value; }

  #[OnStop]
  fn destroy(&self) { *self.last.lock().unwrap() = 0; }
}
