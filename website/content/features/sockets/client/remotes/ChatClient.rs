use crate::models::ChatMessage;

#[HTTPClient("/chat")]
trait ChatClient {
    #[SocketRequest]
    fn receive(&self, message: ChatMessage);
}
