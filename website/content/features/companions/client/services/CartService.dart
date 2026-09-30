import 'models/CartTotal.dart';

@Service
class CartService {
  @Needs
  final ApplicationEventPublisher applicationEventPublisher;
  int total = 0;

  void add() {
    total += 1;
    applicationEventPublisher.publishEvent(CartTotal(total), true);
  }
}
