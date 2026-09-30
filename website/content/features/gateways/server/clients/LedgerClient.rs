#[HTTPClient]
trait LedgerClient {
    #[Gateway({"class":"server/services/LedgerService.py"})]
    fn balance(&self, account: BehaviorValue) -> BehaviorResult<BehaviorValue>;

    #[Gateway({"command":["python3","server/helpers/forecast.py"]})]
    fn forecast(&self, months: BehaviorValue) -> BehaviorResult<BehaviorValue>;
}
