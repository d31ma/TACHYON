// @Service
type ChatService struct {
	// @Needs
	chatClient *ChatClient
	Messages   []ChatMessage
}

// @OnEvent
func (service *ChatService) AddMessage(message ChatMessage) {
	service.Messages = append(service.Messages, message)
	if len(service.Messages) > 50 {
		service.Messages = service.Messages[len(service.Messages)-50:]
	}
}

func (service *ChatService) SayHello() error {
	return service.chatClient.Send(ChatMessage{Text: "Hello from a tab"})
}
