#[SocketRoute("/chat")]
#[Public]
class ChatController
  # @param session [SocketSession]
  # @return [void]
  #[OnOpen]
  def on_open(session)
    session.send(ChatMessage.new(text: "Welcome to the room"))
  end

  # @param session [SocketSession]
  # @param message [ChatMessage]
  # @return [void]
  #[OnMessage]
  def on_message(session, message)
    send_to_everyone(session.open_sessions, message)
  end

  private

  # @param peers [Array<SocketPeer>]
  # @param message [ChatMessage]
  # @return [void]
  def send_to_everyone(peers, message)
    peers.each { |peer| peer.send(message) }
  end
end
