@RequestRoute('/api/greeting')
@Public
export default class GreetingController {
  static _cacheControl = 'cache-control'
  static _noStore = 'no-store'
  static _answer = 'actual registered Yon answer'

  /** @returns {Response} */
  @GetRoute
  getGreeting() {
    return GreetingController._response({ message: GreetingController._answer })
  }

  /** @returns {Response} */
  @GetRoute('/nullable')
  getNullable() {
    return GreetingController._response({ message: null })
  }

  /** @returns {Response} */
  @GetRoute('/missing')
  getMissing() {
    return GreetingController._response({})
  }

  /** @returns {Response} */
  @GetRoute('/wrong-type')
  getWrongType() {
    return GreetingController._response({ message: 42 })
  }

  /** @returns {Response} */
  @GetRoute('/extra')
  getExtra() {
    return GreetingController._response({ message: GreetingController._answer, extra: true })
  }

  /** @returns {Promise<Response>} */
  @GetRoute('/slow')
  async getSlow() {
    await GreetingController._delay()
    return GreetingController._response({ message: 'late registered Yon answer' })
  }

  /** @param {Record<string, unknown>} value @returns {Response} */
  static _response(value) {
    return Response.json(value, { headers: { [GreetingController._cacheControl]: GreetingController._noStore } })
  }

  /** @returns {Promise<void>} */
  static _delay() {
    return new Promise(resolve => setTimeout(resolve, 500))
  }
}
