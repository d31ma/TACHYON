#[Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})]
struct ProductQueriesStore {
    #[Needs]
    behaviors: BehaviorContext,
}

impl ProductQueriesStore {
    fn find_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue> {}
    fn count_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue> {}
    fn exists_by_id(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {}

    #[Query({"operation":"find","where":[{"field":"status","operator":"eq","argument":0}],"orderBy":[]})]
    fn selected(&self, status: BehaviorValue, #[PageDefaults] page: BehaviorValue) -> BehaviorResult<BehaviorValue> {}
}

