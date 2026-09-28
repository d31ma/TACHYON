import models.Report

@RequestRoute("/report")
@Public
class ReportController {
    @Needs
    val reportClient: ReportClient

    @GetRoute
    fun getReport(): Report = Report(reportClient.summarize("2026-Q3"), reportClient.forecast(2L))
}
