#[Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})]
class EntryStore
  # @param status [String]
  # @return [Array<Hash>]
  def find_by_status(status)
  end

  # @param status [String]
  # @return [Integer]
  def count_by_status(status)
  end
end
