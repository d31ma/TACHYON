#[Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})]
class ProductsStore < Repository
  def find_by_status(status)
  end

  def delete_by_status(status)
  end

  def count_by_status(status)
  end

  #[Query({"operation":"exists","where":[{"field":"status","operator":"eq","argument":0}],"orderBy":[]})]
  def available(status)
  end
end
