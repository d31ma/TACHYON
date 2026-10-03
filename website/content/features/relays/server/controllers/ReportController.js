@RequestRoute("/report")
@Public
class ReportController {
  /**
   * @param {Request} request
   * @returns {Promise<Response>}
   */
  @GetRoute
  getReport(request) {
    return relay(['perl', 'server/relays/report.pl'], request)
  }
}
