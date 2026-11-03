@Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})
abstract interface class ProductStore implements Repository<ProductEntity, int> {
  Future<Object?> findByStatus(String status);

  Future<Object?> countByStatus(String status);

  @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
  Future<Object?> excluding(String status, @PageDefaults Map<String, Object?> page);
}
