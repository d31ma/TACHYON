@Store(ProductEntity.class, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
class ProductStore {
    @Needs
    final Behaviors.Context behaviors;
    Object findByStatus(String status) {}
    Object countByStatus(String status) {}
    @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
    Object excluding(String status, @PageDefaults java.util.Map<String, Object> page) {}
    Object bounded(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) java.util.Map<String, Object> request) { return request; }
}
