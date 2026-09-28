@HTTPClient
class ReportClient {
    @Gateway({"class":"server/services/ReportService.py"})
    fun summarize(period: Any?): Any? {}

    @Gateway({"command":["python3","server/helpers/forecast.py"]})
    fun forecast(months: Any?): Any? {}
}
