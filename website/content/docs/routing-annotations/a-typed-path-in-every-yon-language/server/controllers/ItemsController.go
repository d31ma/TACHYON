// @RequestRoute("/items")
// @Public
type ItemsController struct {
}

// @GetRoute("/{id}")
func (controller *ItemsController) GetItem(
	// @FromPath
	id int64,
) Item {
	return Item{ID: id}
}
