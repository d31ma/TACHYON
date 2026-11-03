@RequestRoute("/report")
@Public
class ReportController {
  @GetRoute
  getReport(request: Request): Promise<Response> {
    return relay(['perl', 'server/relays/report.pl'], request)
  }
}
