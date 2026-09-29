from typing import Protocol

@http_client
class ForecastClient(Protocol):
    @gateway({"class":"server/services/ReportService.java","method":"summarize"})
    def summary(self, period: str) -> object: ...

    @gateway({"command":["python3","server/helpers/forecast.py"]})
    def forecast(self, months: int) -> object: ...
