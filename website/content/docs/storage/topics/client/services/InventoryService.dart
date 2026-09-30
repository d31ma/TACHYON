import 'dart:math';

import 'models/InventoryRecord.dart';

@Service
class InventoryService {
  List<InventoryRecord> records = [];

  @OnEvent
  void addRecord(InventoryRecord record) {
    records = [...records.skip(max(0, records.length - 63)), record];
  }
}
