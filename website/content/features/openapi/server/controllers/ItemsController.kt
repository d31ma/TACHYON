import models.Item

@RequestRoute("/items")
@Public
class ItemsController {
    @GetRoute("/{id}")
    fun getItem(@FromPath id: Long): Item = Item("anvil", id)

    @PatchRoute("/{id}")
    fun patchItem(@FromPath id: Long, @FromBody item: Item): Item = Item(item.name, id)
}
