// @Service
type InventoryService struct {
	Products []Product
}

// @OnStart
func (inventory *InventoryService) Start() {
	inventory.Products = []Product{{Name: "Anvil"}, {Name: "Rope"}}
}
