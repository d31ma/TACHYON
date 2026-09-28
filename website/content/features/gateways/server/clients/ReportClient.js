@HTTPClient
class ReportClient {
  /** @param {string} period */
  @Gateway({"class":"server/services/ReportService.java"})
  async summarize(period) {}

  /** @param {number} months */
  @Gateway({"command":["python3","server/helpers/forecast.py"]})
  async forecast(months) {}
}
