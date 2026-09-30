import type { InventoryRecord } from '/models/InventoryRecord.ts'

@Service
export default class InventoryService {
  records: InventoryRecord[] = []

  @OnEvent
  addRecord(record: InventoryRecord): void {
    this.records = [...this.records.slice(-63), record]
  }
}
