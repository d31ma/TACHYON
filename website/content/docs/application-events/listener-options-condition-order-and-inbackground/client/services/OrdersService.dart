import 'models/PlacedOrder.dart';
import 'models/SettledOrder.dart';

@Service
class OrdersService {
  @Needs
  final ApplicationEventPublisher applicationEventPublisher;
  List<Object?> log = [];
  int settled = 0;
  Object? audited = 0;

  void place(int total) {
    applicationEventPublisher.publishEvent(PlacedOrder('paid', total));
  }

  @OnEvent({"id":"orders-log"})
  @Order(-10)
  void record(PlacedOrder order) {
    log = [...log, order.status];
  }

  @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
  SettledOrder settleLargeOrder(PlacedOrder order) {
    settled += 1;
    return SettledOrder(order.total);
  }

  @OnEvent
  @InBackground
  Future<void> audit(SettledOrder order) async {
    audited = order.total;
  }
}
