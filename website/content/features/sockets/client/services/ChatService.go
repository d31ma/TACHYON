// @Service
type ChatService struct {
	// @Needs
	chatClient *ChatClient
	Messages   []ChatMessage
}

// @OnEvent
func (service *ChatService) AddMessage(message ChatMessage) {
	service.Messages = service.recent(append(service.Messages, message))
}

func (service *ChatService) SayHello() error {
	return service.chatClient.Send(ChatMessage{Text: "Hello from a tab"})
}

func (ChatService) messageLimit() int { return 50 }

func (service *ChatService) recent(messages []ChatMessage) []ChatMessage {
	if len(messages) > service.messageLimit() {
		return messages[len(messages)-service.messageLimit():]
	}
	return messages
}
