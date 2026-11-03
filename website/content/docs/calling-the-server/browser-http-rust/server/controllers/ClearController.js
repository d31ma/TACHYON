/** @import { Greeting } from '/models/Greeting.ts' */
@RequestRoute("/api/clear")
@Public
export default class ClearController {
  /** @returns {Greeting} */
  @PostRoute
  postGreeting() {
    return { message: 'Cleared' };
  }
}
