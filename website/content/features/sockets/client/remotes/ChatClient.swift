@HTTPClient("/chat")
protocol ChatClient {
    @SocketRequest
    func receive(_ message: ChatMessage)
}
