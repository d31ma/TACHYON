#[HTTPClient]
trait ReportClient {
    #[Gateway({"class":"server/services/ReportService.py"})]
    fn summarize(&self, period: BehaviorValue) -> BehaviorResult<BehaviorValue>;

    #[Gateway({"command":["python3","server/helpers/forecast.py"]})]
    fn forecast(&self, months: BehaviorValue) -> BehaviorResult<BehaviorValue>;
}
