@HTTPClient
abstract interface class LedgerClient {
  @Gateway({"class":"server/services/LedgerService.py"})
  Future<Object?> balance(Object? account);

  @Gateway({"command":["python3","server/helpers/forecast.py"]})
  Future<Object?> forecast(Object? months);
}
