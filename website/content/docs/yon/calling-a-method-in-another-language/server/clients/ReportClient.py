@h_t_t_p_client
class ReportClient:
    @gateway({"class":"server/services/ReportService.js"})
    def summarize(self, period: str) -> None: ...

    @gateway({"command":["python3","server/helpers/forecast.py"]})
    def forecast(self, months: int) -> None: ...
