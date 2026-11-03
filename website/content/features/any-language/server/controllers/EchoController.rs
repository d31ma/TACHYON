use crate::models::Message;

#[RequestRoute("/echo")]
#[Public]
struct EchoController {}
impl EchoController {
    fn new() -> Self { Self {} }

    #[PostRoute]
    fn post_echo(&self, #[FromBody] message: Message) -> Message {
        message
    }
}
