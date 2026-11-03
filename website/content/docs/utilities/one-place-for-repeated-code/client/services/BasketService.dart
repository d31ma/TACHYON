@Service
class BasketService {
  final int _total = 1250;

  String totalLabel() {
    return PriceUtils.label(_total);
  }
}
