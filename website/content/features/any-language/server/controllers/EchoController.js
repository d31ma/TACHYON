/** @import { Message } from '/models/Message.ts' */
@RequestRoute("/echo")
@Public
class EchoController {
  /**
   * @param {Message} message
   * @returns {Message}
   */
  @PostRoute
  postEcho(@FromBody message) {
    return message
  }
}
