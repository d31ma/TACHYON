#[RequestRoute("/inventory")]
#[Public]
class InventoryController
  # @param record [InventoryRecord]
  # @return [InventoryRecord]
  #[PostRoute]
  def post_record(
    #[FromBody]
    record
  )
    ApplicationEventPublisher.new.publish_event(record)
    record
  end
end
