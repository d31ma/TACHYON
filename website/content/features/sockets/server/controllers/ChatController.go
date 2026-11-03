import "errors"

// @SocketRoute("/chat")
// @Public
type ChatController struct{}

// @OnOpen
func (controller *ChatController) OnOpen(session *SocketSession) error {
	return session.Send(ChatMessage{Text: "Welcome to the room"})
}

// @OnMessage
func (controller *ChatController) OnMessage(session *SocketSession, message ChatMessage) error {
	return controller.sendToEveryone(session.OpenSessions(), message)
}

func (controller *ChatController) sendToEveryone(peers []SocketPeer, message ChatMessage) error {
	failures := []error{}
	for _, peer := range peers {
		failures = append(failures, peer.Send(message))
	}
	return errors.Join(failures...)
}
