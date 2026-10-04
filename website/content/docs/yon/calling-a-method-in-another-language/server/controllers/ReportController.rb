#[RequestRoute("/report")]
#[Public]
class ReportController
  #[Needs]
  # @return [ReportClient]
  attr_reader :report_client

  # @return [Report]
  #[GetRoute]
  def get_report
    Report.new(summary: report_client.summarize("2026-Q3"), forecast: report_client.forecast(2))
  end
end
