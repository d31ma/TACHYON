from typing import Protocol

@http_client
class ReportClient(Protocol):
    @gateway({"class":"server/services/ReportService.js"})
    def summarize(self, period: str) -> object: ...

    @gateway({"command":["python3","server/helpers/forecast.py"]})
    def forecast(self, months: int) -> object: ...
