@Store(ProductEntity::class, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"RuntimeAdaptersSetup"})
interface ProductQueriesStore : Repository<ProductEntity, String> {
    fun findByStatus(status: String): Any?
    fun deleteByStatus(status: String): Any?
    fun countByStatus(status: String): Any?
    fun existsById(id: String): Any?
    @Query({"operation":"find","where":[{"field":"status","operator":"in","argument":0}],"orderBy":[{"field":"id","direction":"asc"}]})
    fun available(statuses: List<String>, @PageDefaults page: Map<String, Any?>): Any?
}
