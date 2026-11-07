@RequestRoute("/api/greeting")
@Public
export default class GreetingController {
  static _message = 'Hello from Yon'

  /** @returns {{message: string}} */
  @GetRoute
  getGreeting() {
    return { message: GreetingController._message }
  }
}
