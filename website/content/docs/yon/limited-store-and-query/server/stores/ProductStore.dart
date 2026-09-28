@Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
class ProductStore {
  @Needs
  final BehaviorContext behaviors;

  Future<Object?> findByStatus(String status) async {}

  Future<Object?> countByStatus(String status) async {}

  @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
  Future<Object?> excluding(String status, @PageDefaults Map<String, Object?> page) async {}

  Object? bounded(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) Map<String, Object?> request) {
    return request;
  }
}
