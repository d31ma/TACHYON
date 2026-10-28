#[yon::setup]
struct LedgerSetup {}
impl LedgerSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        let mut configuration = BehaviorConfiguration::default();
        configuration.entities.insert("ledger".into(), std::sync::Arc::new(LedgerAdapterDelegate::default()));
        configuration.authorize_entity = Some(std::sync::Arc::new(|_command, _execution| Ok(true)));
        Ok(configuration)
    }
}
