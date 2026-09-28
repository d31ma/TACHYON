@RequestRoute("/api/status")
@Public
export class StatusController {
  @GetRoute
  getStatus() { return 'ok'; }
}
