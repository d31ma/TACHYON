/** @import { ChatMessage } from '/models/ChatMessage.ts' */
@SocketRoute('/chat')
@Public
class ChatController {
  /**
   * @param {SocketSession} session
   * @returns {Promise<void>}
   */
  @OnOpen
  onOpen(session) {
    return session.send({ text: 'Welcome to the room' })
  }

  /**
   * @param {SocketSession} session
   * @param {ChatMessage} message
   */
  @OnMessage
  async onMessage(session, message) {
    await this.#sendToEveryone(await session.openSessions(), message)
  }

  /**
   * @param {SocketPeer[]} peers
   * @param {ChatMessage} message
   */
  async #sendToEveryone(peers, message) {
    for (const peer of peers) await peer.send(message)
  }
}
