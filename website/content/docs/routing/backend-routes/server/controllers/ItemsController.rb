#[RequestRoute("/items")]
#[Public]
class ItemsController
  # @param id [String]
  # @return [Item]
  #[GetRoute("/{id}")]
  def get_item(
    #[FromPath]
    id
  )
    Item.new(name: id)
  end

  # @param item [Item]
  # @return [Item]
  #[PostRoute]
  #[ResponseStatus(201)]
  def post_item(
    #[FromBody]
    item
  )
    item
  end
end
