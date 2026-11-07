@Service
final class ChatService {
    @Needs
    private let chatClient: ChatClient
    var messages: [ChatMessage] = []

    @OnEvent
    func addMessage(_ message: ChatMessage) {
        messages = Array((messages + [message]).suffix(messageLimit()))
    }

    func sayHello() async throws {
        try await chatClient.send(ChatMessage(text: "Hello from a tab"))
    }

    private func messageLimit() -> Int { 50 }
}
