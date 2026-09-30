#[HTTPClient]
class ReportClient
  # @param period [String]
  # @return [void]
  #[Gateway({"class":"server/services/ReportService.py"})]
  def summarize(period)
  end

  # @param months [Integer]
  # @return [void]
  #[Gateway({"command":["python3","server/helpers/forecast.py"]})]
  def forecast(months)
  end
end
