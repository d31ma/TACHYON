import models.InventoryRecord

@Service
class InventoryService {
  var records: List<InventoryRecord> = listOf()

  @OnEvent
  fun addRecord(record: InventoryRecord) {
    records = records.takeLast(63) + record
  }
}
