#[RequestRoute("/report")]
#[Public]
class ReportController
  # @param request [Hash]
  # @return [Hash]
  #[GetRoute]
  def get_report(request)
    relay(["perl", "server/relays/report.pl"], request)
  end
end
