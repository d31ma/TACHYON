#[Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})]
struct EntryStore {
    #[Needs]
    behaviors: BehaviorContext,
}
impl EntryStore {
    fn find_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue> {}
    fn count_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue> {}

    #[AllOrNothing({"adapter":"ledger"})]
    fn settle(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let open = self.find_by_status(status.clone())?;
        let total = self.count_by_status(status)?;
        Ok(BehaviorValue::object([("open", open), ("total", total)]))
    }
}
