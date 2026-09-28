import 'package:shelf/shelf.dart';

@RequestRoute("/report")
@Public
class ReportController {
  @GetRoute
  Future<Response> getReport(Request request) =>
      relay(['ruby', 'server/helpers/ReportDelegate.rb'], request);
}
