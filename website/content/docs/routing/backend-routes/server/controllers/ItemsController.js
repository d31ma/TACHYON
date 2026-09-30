@RequestRoute("/items")
@Public
class ItemsController {
  /**
   * @param {string} id
   * @returns {Item}
   */
  @GetRoute("/{id}")
  getItem(@FromPath id) {
    return { name: id }
  }

  /**
   * @param {Item} item
   * @returns {Item}
   */
  @PostRoute
  @ResponseStatus(201)
  postItem(@FromBody item) {
    return item
  }
}
