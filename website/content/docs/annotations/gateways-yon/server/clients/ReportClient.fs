[<HTTPClient>]
type ReportClient() =
    [<Gateway({"class":"server/services/ReportService.py","method":"summarize"})>]
    abstract Summarize: period: BehaviorJSON -> BehaviorJSON

    [<Gateway({"command":["python3","server/helpers/forecast.py"]})>]
    abstract Forecast: months: BehaviorJSON -> BehaviorJSON
