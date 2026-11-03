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
    Item.new(name: "anvil", level: id)
  end

  # @param id [Integer]
  # @param item [Item]
  # @return [Item]
  #[PatchRoute("/{id}")]
  def patch_item(
    #[FromPath]
    id,
    #[FromBody]
    item
  )
    Item.new(name: item.name, level: id)
  end
end
