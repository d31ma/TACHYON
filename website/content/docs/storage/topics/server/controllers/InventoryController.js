/** @import { InventoryRecord } from '/models/InventoryRecord.js' */
@RequestRoute("/inventory")
@Public
class InventoryController {
  /** @type {ApplicationEventPublisher} */
  @Needs
  applicationEventPublisher;

  /**
   * @param {InventoryRecord} record
   * @returns {InventoryRecord}
   */
  @PostRoute
  postRecord(@FromBody record) {
    this.applicationEventPublisher.publishEvent(record)
    return record
  }
}
