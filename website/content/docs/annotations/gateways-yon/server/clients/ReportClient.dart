@HTTPClient
class ReportClient {
  @Gateway({"class":"server/services/ReportService.py"})
  Future<Object?> summarize(Object? period) async {}

  @Gateway({"command":["python3","server/helpers/forecast.py"]})
  Future<Object?> forecast(Object? months) async {}
}
