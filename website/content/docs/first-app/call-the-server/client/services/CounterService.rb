#[Service]
class CounterService
  #[Needs]
  # @return [GreetingClient]
  attr_reader :greeting_client

  # @return [Integer]
  attr_reader :count
  # @return [String]
  attr_reader :message

  def initialize
    @count = 0
    @message = ""
  end

  # @return [void]
  def increment
    @count += 1
  end

  # @return [void]
  def ask_server
    @message = greeting_client.get_greeting.message
  end
end
