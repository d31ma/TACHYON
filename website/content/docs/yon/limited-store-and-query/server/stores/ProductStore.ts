@Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
class ProductStore {
  @Needs
  behaviors!: BehaviorContext;
  async findByStatus(status: string): Promise<unknown> {}
  async countByStatus(status: string): Promise<unknown> {}
  @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
  async excluding(status: string, @PageDefaults page: unknown): Promise<unknown> {}
  bounded(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) request: unknown): unknown { return request; }
}
