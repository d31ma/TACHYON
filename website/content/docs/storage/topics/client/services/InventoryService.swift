@Service
final class InventoryService {
  var records: [InventoryRecord] = []

  @OnEvent
  func addRecord(_ record: InventoryRecord) {
    records = Array(records.suffix(63)) + [record]
  }
}
