#[RequestRoute("/probe")]
#[Public]
class ProbeController
  #[Needs]
  # @return [ProbeService]
  attr_reader :probe_service

  #[Needs]
  # @return [PublishService]
  attr_reader :publish_service

  # @return [ProbeReady, nil]
  #[GetRoute]
  def get_probe
    probe_service.last
  end
end
