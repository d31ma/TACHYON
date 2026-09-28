[HTTPClient]
class ReportClient {
    [Gateway({"class":"server/services/ReportService.py","method":"summarize"})]
    public BehaviorJSON Summarize(BehaviorJSON period) {}

    [Gateway({"command":["python3","server/helpers/forecast.py"]})]
    public BehaviorJSON Forecast(BehaviorJSON months) {}
}
