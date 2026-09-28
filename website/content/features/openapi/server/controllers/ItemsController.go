// @RequestRoute("/items")
// @Public
type ItemsController struct {
}

// @GetRoute("/{id}")
func (controller *ItemsController) GetItem(
	// @FromPath
	id int64,
) Item {
	return Item{Name: "anvil", Level: id}
}

// @PatchRoute("/{id}")
func (controller *ItemsController) PatchItem(
	// @FromPath
	id int64,
	// @FromBody
	item Item,
) Item {
	return Item{Name: item.Name, Level: id}
}
