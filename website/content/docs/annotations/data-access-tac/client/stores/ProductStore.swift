@Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})
protocol ProductStore {
    func findByStatus(_ status: BehaviorJSON) async throws -> BehaviorJSON
    func countByStatus(_ status: BehaviorJSON) async throws -> BehaviorJSON
    @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
    func excluding(_ status: BehaviorJSON, @PageDefaults _ page: BehaviorJSON) async throws -> BehaviorJSON
}
