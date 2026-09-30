#[Store(BillingEntity, {"fields":["id"],"defaultSize":20,"maxSize":100})]
class BillingStore
  # @param id [Integer]
  # @return [void]
  def exists_by_id(id)
  end
end
