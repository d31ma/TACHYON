#[Service]
class ActivityService
  # @return [String]
  attr_reader :state

  def initialize
    @state = "foreground"
  end

  # @param changed [ApplicationStateChanged]
  # @return [void]
  #[OnEvent({"replay":true})]
  def application_state_changed(changed)
    @state = changed.state
  end
end
