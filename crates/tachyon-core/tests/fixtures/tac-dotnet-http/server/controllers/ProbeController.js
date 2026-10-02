@RequestRoute("/api/probe")
@Public
export default class ProbeController {
  /** @returns {{message: string}} */
  @GetRoute
  read() {
    return { message: 'actual Yon answer' }
  }
}
