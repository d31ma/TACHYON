@Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})
export default class ProductStore extends Repository<ProductEntity, number> {
  async findByStatus(status: string): Promise<ProductEntity[] | void> {}
  async countByStatus(status: string): Promise<number | void> {}
  @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
  async excluding(status: string, @PageDefaults page: BehaviorPageInput): Promise<ProductEntity[] | void> {}
}
