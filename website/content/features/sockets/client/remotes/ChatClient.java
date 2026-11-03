import models.ChatMessage;

@HTTPClient("/chat")
interface ChatClient {
    @SocketRequest
    void receive(ChatMessage message);
}
