import 'models/InventoryRecord.dart';

@RequestRoute("/inventory")
@Public
class InventoryController {
  @Needs
  final ApplicationEventPublisher applicationEventPublisher;

  @PostRoute
  InventoryRecord postRecord(@FromBody InventoryRecord record) {
    applicationEventPublisher.publishEvent(record);
    return record;
  }
}
