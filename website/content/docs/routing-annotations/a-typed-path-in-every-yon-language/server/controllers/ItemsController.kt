import models.Item

@RequestRoute("/items")
@Public
class ItemsController {
    @GetRoute("/{id}")
    fun getItem(@FromPath id: Long): Item = Item(id)
}
