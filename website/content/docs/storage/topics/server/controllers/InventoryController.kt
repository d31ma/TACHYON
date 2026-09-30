import models.InventoryRecord

@RequestRoute("/inventory")
@Public
class InventoryController {
    @Needs
    lateinit var applicationEventPublisher: ApplicationEventPublisher

    @PostRoute
    fun postRecord(@FromBody record: InventoryRecord): InventoryRecord {
        applicationEventPublisher.publishEvent(record)
        return record
    }
}
