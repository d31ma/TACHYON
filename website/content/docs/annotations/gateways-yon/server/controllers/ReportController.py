from typing import Annotated
from models import Report


@request_route("/report")
@public
class ReportController:
    report_client: Annotated[ReportClient, needs]

    @get_route
    def get_report(self) -> Report:
        return Report(self.report_client.summarize("2026-Q3"), self.report_client.forecast(2))
