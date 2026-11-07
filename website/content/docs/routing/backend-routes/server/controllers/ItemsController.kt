import models.Item

@RequestRoute("/items")
@Public
class ItemsController {
    @GetRoute("/{id}")
    fun getItem(@FromPath id: String): Item = Item(id)

    @PostRoute
    @ResponseStatus(201)
    fun postItem(@FromBody item: Item): Item = item
}
