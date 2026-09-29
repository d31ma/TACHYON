import type { ChatMessage } from '/models/ChatMessage.ts'

@SocketRoute('/chat')
@Public
class ChatController {
  @OnOpen
  onOpen(session: SocketSession) {
    return session.send({ text: 'Welcome to the room' })
  }

  @OnMessage
  async onMessage(session: SocketSession, message: ChatMessage) {
    await this.sendToEveryone(await session.openSessions(), message)
  }

  private async sendToEveryone(peers: SocketPeer[], message: ChatMessage) {
    for (const peer of peers) await peer.send(message)
  }
}
