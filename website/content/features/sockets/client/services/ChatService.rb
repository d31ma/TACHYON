#[Service]
class ChatService
  #[Needs]
  # @return [ChatClient]
  attr_reader :chat_client

  # @return [Array<ChatMessage>]
  attr_reader :messages

  def initialize
    @messages = []
  end

  # @param message [ChatMessage]
  # @return [void]
  #[OnEvent]
  def add_message(message)
    @messages = [*@messages.last(message_limit - 1), message]
  end

  # @return [void]
  def say_hello
    chat_client.send_message(ChatMessage.new(text: "Hello from a tab"))
  end

  private

  # @return [Integer]
  def message_limit
    50
  end
end
