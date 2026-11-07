@HTTPClient
abstract interface class ReportClient {
  @Gateway({"class":"server/services/ReportService.py"})
  Future<Object?> summarize(Object? period);

  @Gateway({"command":["python3","server/helpers/forecast.py"]})
  Future<Object?> forecast(Object? months);
}
