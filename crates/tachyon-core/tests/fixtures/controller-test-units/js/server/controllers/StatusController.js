@RequestRoute("/api/status")
@Public
export class StatusController {
  /** @returns {string} */
  @GetRoute
  getStatus() { return 'ok'; }
}
