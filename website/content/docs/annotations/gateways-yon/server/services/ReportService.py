@service
class ReportService:
    def summarize(self, period: str) -> dict[str, object]:
        return {"period": period, "total": 42, "language": "python"}
