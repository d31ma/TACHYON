#[RequestRoute("/report")]
#[Public]
class ReportController
  # @param request [Hash]
  # @return [Hash]
  #[GetRoute]
  def get_report(request)
    relay(["ruby", "server/helpers/ReportDelegate.rb"], request)
  end
end
