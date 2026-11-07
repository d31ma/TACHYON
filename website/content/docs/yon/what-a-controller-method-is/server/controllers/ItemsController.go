// @RequestRoute("/items")
// @Public
type ItemsController struct {
}

// @GetRoute
func (controller *ItemsController) GetItems() ItemList {
	return ItemList{Items: []Item{}}
}

// @PostRoute
// @ResponseStatus(201)
func (controller *ItemsController) PostItem(
	// @FromBody
	item Item,
) Item {
	return item
}
