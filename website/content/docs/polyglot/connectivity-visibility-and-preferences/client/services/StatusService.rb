#[Service]
class StatusService
  # @return [Boolean]
  attr_reader :online
  # @return [String]
  attr_reader :scheme

  def initialize
    @online = true
    @scheme = "light"
  end

  # @param changed [ConnectivityChanged]
  # @return [void]
  #[OnEvent({"replay":true})]
  def connectivity_changed(changed)
    @online = changed.online
  end

  # @param changed [PreferencesChanged]
  # @return [void]
  #[OnEvent({"replay":true})]
  def preferences_changed(changed)
    @scheme = changed.scheme
  end
end
