#[Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})]
class ProductStore < Repository
  # @param status [String]
  # @return [Array<Hash>]
  def find_by_status(status)
  end

  # @param status [String]
  # @return [Integer]
  def count_by_status(status)
  end

  # @param status [String]
  # @param page [Hash]
  # @return [Array<Hash>]
  #[Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})]
  def excluding(
    status,
    #[PageDefaults]
    page
  )
  end
end
