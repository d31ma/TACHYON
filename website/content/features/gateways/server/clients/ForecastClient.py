@h_t_t_p_client
class ForecastClient:
    @gateway({"class":"server/services/ReportService.java","method":"summarize"})
    def summary(self, period: str) -> None: ...

    @gateway({"command":["python3","server/helpers/forecast.py"]})
    def forecast(self, months: int) -> None: ...
