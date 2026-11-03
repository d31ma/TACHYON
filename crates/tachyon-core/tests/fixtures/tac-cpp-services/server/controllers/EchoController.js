/** @import { Greeting } from '/models/Greeting.ts' */
@RequestRoute("/api/echo")
@Public
export default class EchoController {
  /**
   * @param {string} id
   * @param {Greeting} greeting
   * @returns {Greeting}
   */
  @PostRoute("/{id}")
  postEcho(@FromPath id, @FromBody greeting) {
    return { message: `${greeting.message} #${id}` }
  }
}
