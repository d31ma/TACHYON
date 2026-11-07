@RequestRoute("/api/probe")
@Public
export default class ProbeController {
  /** @returns {{message: string}} */
  @GetRoute
  getProbe() {
    return { message: 'actual Yon answer' }
  }
}
