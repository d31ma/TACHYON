import models.InventoryRecord;

@RequestRoute("/inventory")
@Public
class InventoryController {
    @Needs
    ApplicationEventPublisher applicationEventPublisher;

    @PostRoute
    InventoryRecord postRecord(@FromBody InventoryRecord record) {
        applicationEventPublisher.publishEvent(record);
        return record;
    }
}
