/** @import { ChatMessage } from '/models/ChatMessage.ts' */
@HTTPClient('/chat')
class ChatClient {
  /** @param {ChatMessage} message */
  @SocketRequest
  receive(message) {}
}
