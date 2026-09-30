#[Service]
class ReleaseService
  # @return [String]
  attr_reader :release

  def initialize
    @release = ""
    @events = ApplicationEventPublisher.new
  end

  # @return [void]
  def announce
    @events.publish_event(ReleaseChanged.new(release: "TACHYON"), true)
  end

  # @param changed [ReleaseChanged]
  # @return [void]
  #[OnEvent({"replay":true})]
  def receive(changed)
    @release = changed.release
  end
end
