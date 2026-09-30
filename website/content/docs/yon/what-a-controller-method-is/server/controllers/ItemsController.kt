import models.Item
import models.ItemList

@RequestRoute("/items")
@Public
class ItemsController {
    @GetRoute
    fun getItems(): ItemList = ItemList(listOf())

    @PostRoute
    @ResponseStatus(201)
    fun postItem(@FromBody item: Item): Item = item
}
