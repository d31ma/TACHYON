@HTTPClient
interface LedgerClient {
    @Gateway({"class":"server/services/LedgerService.py"})
    Object balance(Object account)

    @Gateway({"command":["python3","server/helpers/forecast.py"]})
    Object forecast(Object months)
}
