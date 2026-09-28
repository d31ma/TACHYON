import 'models/PlacedOrder.dart';

@Service
class CheckoutService {
  @Needs
  final ApplicationEventPublisher applicationEventPublisher;
  int placed = 0;

  @OnStart
  void start() {
    place();
  }

  void place() {
    placed += 1;
    applicationEventPublisher.publishEvent(const PlacedOrder('paid', 25), true);
  }

  @OnStop
  void stop() {
    placed = 0;
  }
}
