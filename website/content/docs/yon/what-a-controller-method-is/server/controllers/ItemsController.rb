#[RequestRoute("/items")]
#[Public]
class ItemsController
  # @return [ItemList]
  #[GetRoute]
  def get_items
    ItemList.new(items: [])
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
