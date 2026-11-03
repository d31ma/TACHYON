#[RequestRoute("/items")]
#[Public]
class ItemsController
  # @param id [Integer]
  # @return [Item]
  #[GetRoute("/{id}")]
  def get_item(
    #[FromPath]
    id
  )
    Item.new(id: id)
  end
end
