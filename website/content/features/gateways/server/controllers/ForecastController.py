from models import Forecast


@request_route("/forecast")
@public
@needs_all()
class ForecastController:
    forecast_client: ForecastClient

    @get_route
    def get_forecast(self) -> Forecast:
        summary = self.forecast_client.summary("2026-Q4")
        return Forecast(summary, self.forecast_client.forecast(2))
