@RequestRoute("/report")
@Public
class ReportController {
  @GetRoute
  getReport(request: Request): Promise<Response> {
    return relay(['ruby', 'server/helpers/ReportDelegate.rb'], request)
  }
}
