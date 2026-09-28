use crate::models::Report;

#[RequestRoute("/report")]
#[Public]
struct ReportController {
    #[Needs]
    report_client: std::sync::Arc<ReportClient>,
}
impl ReportController {
    #[GetRoute]
    fn get_report(&self) -> Result<Report, YonResourceError> {
        let summary = self.report_client.summarize("2026-Q3".into()).map_err(|_| YonResourceError)?;
        let forecast = self.report_client.forecast(BehaviorValue::Number(2.0)).map_err(|_| YonResourceError)?;
        Ok(Report { summary: Self::json(&summary)?, forecast: Self::json(&forecast)? })
    }

    fn json(value: &BehaviorValue) -> Result<Json, YonResourceError> {
        let text = value.to_json(1_048_576).map_err(|_| YonResourceError)?;
        Json::parse(&text).map_err(|_| YonResourceError)
    }
}
