@Service
class ChatService {
  static const _messageLimit = 50;

  @Needs
  final ChatClient chatClient;
  List<ChatMessage> messages = [];

  @OnEvent
  void addMessage(ChatMessage message) {
    messages = _latest([...messages, message]);
  }

  Future<void> sayHello() => chatClient.send(const ChatMessage('Hello from a tab'));

  List<ChatMessage> _latest(List<ChatMessage> all) =>
      all.length > _messageLimit ? all.sublist(all.length - _messageLimit) : all;
}
