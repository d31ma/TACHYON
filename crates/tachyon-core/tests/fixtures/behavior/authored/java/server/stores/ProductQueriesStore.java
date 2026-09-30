@Store(ProductEntity.class, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"RuntimeAdaptersSetup"})
interface ProductQueriesStore extends Repository<ProductEntity, String> {
    Object findByStatus(String status);
    Object deleteByStatus(String status);
    Object countByStatus(String status);
    Object existsById(String id);
    @Query({"operation":"find","where":[{"field":"status","operator":"in","argument":0}],"orderBy":[{"field":"id","direction":"asc"}]})
    Object available(java.util.List<String> statuses, @PageDefaults java.util.Map<String, Object> page);
}
