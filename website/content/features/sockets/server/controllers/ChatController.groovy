import models.ChatMessage

@SocketRoute("/chat")
@Public
class ChatController {
    @OnOpen
    void onOpen(SocketSession session) throws Exception {
        session.send(new ChatMessage("Welcome to the room"))
    }

    @OnMessage
    void onMessage(SocketSession session, ChatMessage message) throws Exception {
        sendToEveryone(session.getOpenSessions(), message)
    }

    private void sendToEveryone(java.util.List<SocketPeer> peers, ChatMessage message) throws Exception {
        for (SocketPeer peer : peers) peer.send(message)
    }
}
