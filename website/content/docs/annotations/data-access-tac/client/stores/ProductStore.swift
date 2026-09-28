@Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
final class ProductStore {
    @Needs
    let behaviors: BehaviorContext
    func findByStatus(_ status: BehaviorJSON) async throws -> BehaviorJSON {}
    func countByStatus(_ status: BehaviorJSON) async throws -> BehaviorJSON {}
    @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
    func excluding(_ status: BehaviorJSON, @PageDefaults _ page: BehaviorJSON) async throws -> BehaviorJSON {}
    func bounded(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) _ request: BehaviorJSON) throws -> BehaviorJSON { return request }
}
