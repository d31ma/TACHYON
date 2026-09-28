#[Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})]
class ProductsStore
  attr_accessor :behaviors

  def find_by_status(status)
  end

  def count_by_status(status)
  end

  #[Query({"operation":"exists","where":[{"field":"status","operator":"eq","argument":0}],"orderBy":[]})]
  def available(status)
  end
end
