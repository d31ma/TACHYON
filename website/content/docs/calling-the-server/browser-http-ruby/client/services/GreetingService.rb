#[Service]
class GreetingService
  #[Needs]
  # @return [GreetingClient]
  attr_reader :greeting_client

  #[Needs]
  # @return [OverrideClient]
  attr_reader :override_client

  # @return [String]
  attr_reader :message, :custom
  # @return [Integer]
  attr_reader :loads, :clears

  def initialize
    @message = ""
    @custom = ""
    @loads = 0
    @clears = 0
  end

  # @return [void]
  def ask
    @loads += 1
    @message = greeting_client.read.message
  end

  # @return [void]
  def clear
    greeting_client.clear
    @clears += 1
  end

  # @return [void]
  def ask_custom
    @custom = override_client.read.message
  end
end
