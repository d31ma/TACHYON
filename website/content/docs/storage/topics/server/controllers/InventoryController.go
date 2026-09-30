// @RequestRoute("/inventory")
// @Public
type InventoryController struct {
	// @Needs
	applicationEventPublisher *ApplicationEventPublisher
}

// @PostRoute
func (controller *InventoryController) PostRecord(
	// @FromBody
	record InventoryRecord,
) InventoryRecord {
	controller.applicationEventPublisher.PublishEvent(record, false)
	return record
}
