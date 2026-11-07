import type { Message } from '/models/Message.ts';

@RequestRoute("/echo")
@Public
class EchoController {
  @PostRoute
  postEcho(@FromBody message: Message): Message {
    return message
  }
}
