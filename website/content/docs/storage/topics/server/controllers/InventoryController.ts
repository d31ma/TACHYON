import type { InventoryRecord } from '/models/InventoryRecord.ts'

@RequestRoute("/inventory")
@Public
class InventoryController {
  @Needs
  readonly applicationEventPublisher!: ApplicationEventPublisher

  @PostRoute
  postRecord(@FromBody record: InventoryRecord): InventoryRecord {
    this.applicationEventPublisher.publishEvent(record)
    return record
  }
}
