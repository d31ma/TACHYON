@HTTPClient
interface ReportClient {
    @Gateway({"class":"server/services/ReportService.py"})
    Object summarize(Object period)

    @Gateway({"command":["python3","server/helpers/forecast.py"]})
    Object forecast(Object months)
}
