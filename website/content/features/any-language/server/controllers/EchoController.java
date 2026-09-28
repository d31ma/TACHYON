import models.Message;

@RequestRoute("/echo")
@Public
class EchoController {
    @PostRoute
    Message postEcho(@FromBody Message message) {
        return message;
    }
}
