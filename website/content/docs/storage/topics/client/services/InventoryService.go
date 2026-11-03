// @Service
type InventoryService struct {
	Records []InventoryRecord
}

// @OnEvent
func (inventory *InventoryService) AddRecord(record InventoryRecord) {
	older := max(0, len(inventory.Records)-63)
	inventory.Records = append(inventory.Records[older:], record)
}
