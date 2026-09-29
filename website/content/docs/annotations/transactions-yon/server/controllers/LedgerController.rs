use crate::models::LedgerReport;

#[RequestRoute("/ledger")]
#[Public]
struct LedgerController {
    #[Needs]
    ledger: std::sync::Arc<LedgerService>,
}
impl LedgerController {
    #[GetRoute]
    fn get_ledger(&self) -> Result<LedgerReport, YonResourceError> {
        let settled = self.ledger.settle("open".into()).map_err(|_| YonResourceError)?;
        let logged_commands = LedgerAdapterHelper::commands().lock().expect("log").clone();
        let commands = BehaviorValue::Array(logged_commands.into_iter().map(BehaviorValue::from).collect());
        Ok(LedgerReport { settled: Self::json(settled), commands: Self::json(commands) })
    }

    fn json(value: BehaviorValue) -> Json {
        Json::parse(&value.to_json(65536).expect("serialize")).expect("parse")
    }
}
