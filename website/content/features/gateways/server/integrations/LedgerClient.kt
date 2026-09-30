@HTTPClient
interface LedgerClient {
    @Gateway({"class":"server/services/LedgerService.py"})
    fun balance(account: Any?): Any?

    @Gateway({"command":["python3","server/helpers/forecast.py"]})
    fun forecast(months: Any?): Any?
}
