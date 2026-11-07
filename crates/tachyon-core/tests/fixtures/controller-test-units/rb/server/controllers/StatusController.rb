#[RequestRoute("/api/status")]
#[Public]
class StatusController
  # @return [String]
  #[GetRoute]
  def get_status
    "ok"
  end
end
