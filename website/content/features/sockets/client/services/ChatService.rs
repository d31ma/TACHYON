#[Service]
pub struct ChatService {
    #[Needs]
    chat_client: std::sync::Arc<ChatClient>,
    pub messages: Vec<ChatMessage>,
}

impl ChatService {
    const MESSAGE_LIMIT: usize = 50;

    #[OnEvent]
    fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
        let extra = self.messages.len().saturating_sub(Self::MESSAGE_LIMIT);
        self.messages.drain(..extra);
    }

    pub async fn say_hello(&mut self) -> BehaviorResult<()> {
        let hello = ChatMessage { text: String::from("Hello from a tab") };
        self.chat_client.send(hello).await
    }
}
