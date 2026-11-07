#[Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})]
trait EntryStore {
    fn find_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue>;
    fn count_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue>;
}
