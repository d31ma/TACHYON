import models.Message

@RequestRoute("/echo")
@Public
class EchoController {
    @PostRoute
    fun postEcho(@FromBody message: Message): Message = message
}
