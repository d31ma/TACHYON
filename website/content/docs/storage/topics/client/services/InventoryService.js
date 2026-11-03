@Service
export default class InventoryService {
  /** @type {InventoryRecord[]} */
  records = []

  /** @param {InventoryRecord} record */
  @OnEvent
  addRecord(record) {
    this.records = [...this.records.slice(-63), record]
  }
}
