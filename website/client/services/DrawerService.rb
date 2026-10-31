#[Service]
class DrawerService
  # @return [Boolean]
  attr_reader :open

  def initialize
    @open = false
  end

  # @param state [String]
  # @return [Boolean]
  def track(state)
    @open = state == "open"
  end
end
