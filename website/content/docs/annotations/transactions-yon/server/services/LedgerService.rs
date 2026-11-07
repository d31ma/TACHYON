#[yon::service]
#[Needs(EntryStore)]
struct LedgerService {
    behaviors: BehaviorContext,
    entries: std::sync::Arc<EntryStore>,
}
impl LedgerService {
    fn new(entries: std::sync::Arc<EntryStore>) -> Self {
        let options = BehaviorOptions::new("ledger", BehaviorSignal::new());
        Self {
            behaviors: Behaviors::create::<LedgerSetup>(options).expect("behavior context"),
            entries,
        }
    }

    #[AllOrNothing({"adapter":"ledger"})]
    fn settle(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let open = self.entries.find_by_status(status.clone())?;
        let total = self.entries.count_by_status(status)?;
        Ok(BehaviorValue::object([("open", open), ("total", total)]))
    }
}
