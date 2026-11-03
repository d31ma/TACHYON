import models.Report

@RequestRoute("/report")
@Public
class ReportController {
    @Needs
    final ReportClient reportClient

    @GetRoute
    Report getReport() {
        return new Report(reportClient.summarize("2026-Q3"), reportClient.forecast(2L))
    }
}
