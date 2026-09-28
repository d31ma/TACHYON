import 'models/PlacedOrder.dart';

@Service
class CheckoutService {
  @Needs
  final ApplicationEventPublisher applicationEventPublisher;

  @OnStart
  void start() {
    applicationEventPublisher.publishEvent(const PlacedOrder('paid', 25));
  }

  @OnStop
  void stop() {}
}
