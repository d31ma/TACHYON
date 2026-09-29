[HTTPClient]
interface ReportClient {
    [Gateway({"class":"server/services/ReportService.py","method":"summarize"})]
    BehaviorJSON Summarize(BehaviorJSON period);

    [Gateway({"command":["python3","server/helpers/forecast.py"]})]
    BehaviorJSON Forecast(BehaviorJSON months);
}
