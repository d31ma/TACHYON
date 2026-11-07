#[HTTPClient("/chat")]
class ChatClient
  # @param message [ChatMessage]
  # @return [void]
  #[SocketRequest]
  def receive(message)
  end
end
