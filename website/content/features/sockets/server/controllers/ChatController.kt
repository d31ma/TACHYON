import models.ChatMessage

@SocketRoute("/chat")
@Public
class ChatController {
    @OnOpen
    fun onOpen(session: SocketSession) {
        session.send(ChatMessage("Welcome to the room"))
    }

    @OnMessage
    fun onMessage(session: SocketSession, message: ChatMessage) {
        sendToEveryone(session.openSessions(), message)
    }

    private fun sendToEveryone(peers: List<SocketPeer>, message: ChatMessage) {
        for (peer in peers) peer.send(message)
    }
}
