// @HTTPClient("/chat")
type ChatClient interface {
	// @SocketRequest
	Receive(message ChatMessage)
}
