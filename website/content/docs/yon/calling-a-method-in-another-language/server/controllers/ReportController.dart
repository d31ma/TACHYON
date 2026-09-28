import 'models/Report.dart';

@RequestRoute("/report")
@Public
class ReportController {
  @Needs
  final ReportClient reportClient;

  @GetRoute
  Future<Report> getReport() async {
    final summary = await reportClient.summarize('2026-Q3');
    final forecast = await reportClient.forecast(2);
    return Report(summary, forecast);
  }
}
