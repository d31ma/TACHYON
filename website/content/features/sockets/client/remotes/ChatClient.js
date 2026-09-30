/** @import { ChatMessage } from '/models/ChatMessage.ts' */
@HTTPClient('/chat')
export default class ChatClient {
  /** @param {ChatMessage} message */
  @SocketRequest
  receive(message) {}
}
