@RequestRoute("/api/slow")
@Public
export default class SlowController {
  /** @returns {Promise<{message: string}>} */
  @GetRoute
  async getSlow() {
    await new Promise(resolve => setTimeout(resolve, 500))
    return { message: 'late Yon answer' }
  }
}
