#[Service]
class InventoryService
  # @return [Array<InventoryRecord>]
  attr_reader :records

  def initialize
    @records = []
  end

  # @param record [InventoryRecord]
  # @return [void]
  #[OnEvent]
  def add_record(record)
    @records = [*@records.last(63), record]
  end
end
