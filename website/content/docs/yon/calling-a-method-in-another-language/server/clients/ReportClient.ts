@HTTPClient
class ReportClient {
  @Gateway({"class":"server/services/ReportService.go"})
  async summarize(period: BehaviorJsonValue): Promise<BehaviorJsonValue> {}

  @Gateway({"command":["python3","server/helpers/forecast.py"]})
  async forecast(months: BehaviorJsonValue): Promise<BehaviorJsonValue> {}
}
