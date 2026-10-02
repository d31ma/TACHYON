@RequestRoute("/api/slow")
@Public
export default class SlowController {
  /** @returns {Promise<{message: string}>} */
  @GetRoute
  async read() {
    await new Promise(resolve => setTimeout(resolve, 500))
    return { message: 'late Yon answer' }
  }
}
