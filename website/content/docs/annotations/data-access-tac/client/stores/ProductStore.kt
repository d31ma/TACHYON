@Store(ProductEntity::class, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})
interface ProductStore : Repository<ProductEntity, Int> {
    fun findByStatus(status: String): Any?
    fun countByStatus(status: String): Any?
    @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
    fun excluding(status: String, @PageDefaults page: Map<String, Any?>): Any?
}
