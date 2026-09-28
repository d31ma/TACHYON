@RequestRoute("/items")
class ItemsController {
  @GetRoute
  getItems() {
    return { items: [] }
  }

  @PostRoute
  @ResponseStatus(201)
  postItem(@FromBody item) {
    return { created: item }
  }
}