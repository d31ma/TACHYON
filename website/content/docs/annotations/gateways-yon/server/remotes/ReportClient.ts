@HTTPClient
export default class ReportClient {
  @Gateway({"class":"server/services/ReportService.go"})
  async summarize(period: BehaviorJsonValue): Promise<BehaviorJsonValue | void> {}

  @Gateway({"command":["python3","server/helpers/forecast.py"]})
  async forecast(months: BehaviorJsonValue): Promise<BehaviorJsonValue | void> {}
}
