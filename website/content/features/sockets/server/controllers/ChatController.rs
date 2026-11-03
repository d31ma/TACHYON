use crate::models::ChatMessage;

#[SocketRoute("/chat")]
#[Public]
struct ChatController {}

impl ChatController {
    fn new() -> Self {
        Self {}
    }

    #[OnOpen]
    fn on_open(&self, session: &SocketSession) {
        session.send(ChatMessage { text: String::from("Welcome to the room") });
    }

    #[OnMessage]
    fn on_message(&self, session: &SocketSession, message: ChatMessage) {
        Self::send_to_everyone(session.open_sessions(), message);
    }

    fn send_to_everyone(peers: Vec<SocketPeer>, message: ChatMessage) {
        for peer in peers {
            peer.send(ChatMessage { text: message.text.clone() });
        }
    }
}
