// @RequestRoute("/items")
// @Public
type ItemsController struct {
}

// @GetRoute("/{id}")
func (controller *ItemsController) GetItem(
	// @FromPath
	id string,
) Item {
	return Item{Name: id}
}

// @PostRoute
// @ResponseStatus(201)
func (controller *ItemsController) PostItem(
	// @FromBody
	item Item,
) Item {
	return item
}
