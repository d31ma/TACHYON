@Store(ProductEntity, {"fields":["id"],"defaultSize":20,"maxSize":100})
class ProductQueriesStore {
  @Needs
  behaviors;
  async findById(id) {}
  async count_by_id(id) {}
  @Query({"operation":"exists","where":[{"field":"id","operator":"eq","argument":0}],"orderBy":[]})
  async available(id) {}
}

