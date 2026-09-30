#[Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})]
trait ProductStore: Repository<ProductEntity, i64> {
    fn find_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue>;
    fn count_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue>;
    #[Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})]
    fn excluding(&self, status: BehaviorValue, #[PageDefaults] page: BehaviorValue) -> BehaviorResult<BehaviorValue>;
}
