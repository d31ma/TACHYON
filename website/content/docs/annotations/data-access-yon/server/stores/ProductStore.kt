@Store(ProductEntity::class, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
class ProductStore {
    @Needs
    val behaviors: Behaviors.Context
    fun findByStatus(status: String): Any? {}
    fun countByStatus(status: String): Any? {}
    @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
    fun excluding(status: String, @PageDefaults page: Map<String, Any?>): Any? {}
    fun bounded(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) request: Map<String, Any?>): Any? { return request }
}
