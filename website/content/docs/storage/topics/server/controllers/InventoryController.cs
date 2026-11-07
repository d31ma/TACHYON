using Models;

[RequestRoute("/inventory")]
[Public]
class InventoryController {
    [Needs]
    readonly ApplicationEventPublisher _applicationEventPublisher;

    [PostRoute]
    public InventoryRecord PostRecord([FromBody] InventoryRecord record) {
        _applicationEventPublisher.PublishEvent(record);
        return record;
    }
}
