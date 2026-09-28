use crate::models::LedgerReport;

#[RequestRoute("/ledger")]
#[Public]
struct LedgerController {}
impl LedgerController {
    fn new() -> Self { Self {} }
    #[GetRoute]
    fn get_ledger(&self) -> Result<LedgerReport, YonResourceError> {
        let options = BehaviorOptions::new("ledger", BehaviorSignal::new());
        let context = Behaviors::create::<LedgerSetup>(options).map_err(|_| YonResourceError)?;
        let entries = EntryStore::new(context.clone());
        let settled = entries.settle("open".into()).map_err(|_| YonResourceError)?;
        let logged_commands = LedgerAdapterHelper::commands().lock().expect("log").clone();
        let commands = BehaviorValue::Array(logged_commands.into_iter().map(BehaviorValue::from).collect());
        context.dispose();
        Ok(LedgerReport { settled: Self::json(settled), commands: Self::json(commands) })
    }

    fn json(value: BehaviorValue) -> Json {
        Json::parse(&value.to_json(65536).expect("serialize")).expect("parse")
    }
}
