import java.util.ArrayList;
import java.util.List;
import models.ChatMessage;

@Service
class ChatService {
    private static final int MESSAGE_LIMIT = 50;

    @Needs
    private final ChatClient chatClient;
    List<ChatMessage> messages = new ArrayList<>();

    @OnEvent
    void addMessage(ChatMessage message) {
        int older = Math.max(0, messages.size() - (MESSAGE_LIMIT - 1));
        List<ChatMessage> kept = new ArrayList<>(messages.subList(older, messages.size()));
        kept.add(message);
        messages = kept;
    }

    void sayHello() {
        chatClient.send(new ChatMessage("Hello from a tab"));
    }
}
