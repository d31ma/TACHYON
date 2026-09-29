#[Store(BillingEntity, {"fields":["id"],"defaultSize":20,"maxSize":100})]
trait BillingStore {
    fn exists_by_id(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue>;
}
