#[Service]
class PublishService
  # @return [void]
  #[OnStart]
  def start
    ApplicationEventPublisher.new.publish_event(ProbeReady.new(value: 7))
  end
end
