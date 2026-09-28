@RequestRoute("/report")
@Public
class ReportController {
  /**
   * @param {Request} request
   * @returns {Promise<Response>}
   */
  @GetRoute
  getReport(request) {
    return relay(['ruby', 'server/helpers/ReportDelegate.rb'], request)
  }
}
