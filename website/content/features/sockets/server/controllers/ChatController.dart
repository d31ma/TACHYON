import 'models/ChatMessage.dart';

@SocketRoute("/chat")
@Public
class ChatController {
  @OnOpen
  Future<void> onOpen(SocketSession session) async {
    await session.send(const ChatMessage('Welcome to the room'));
  }

  @OnMessage
  Future<void> onMessage(SocketSession session, ChatMessage message) async {
    await _sendToEveryone(await session.openSessions(), message);
  }

  Future<void> _sendToEveryone(List<SocketPeer> peers, ChatMessage message) async {
    for (final peer in peers) {
      await peer.send(message);
    }
  }
}
