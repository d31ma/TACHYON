using Models;

[RequestRoute("/echo")]
[Public]
class EchoController {
    [PostRoute]
    public Message PostEcho([FromBody] Message message) {
        return message;
    }
}
