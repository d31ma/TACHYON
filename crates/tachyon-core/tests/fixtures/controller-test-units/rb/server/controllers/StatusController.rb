#[RequestRoute("/api/status")]
#[Public]
class StatusController
  #[GetRoute]
  def get_status
    "ok"
  end
end
