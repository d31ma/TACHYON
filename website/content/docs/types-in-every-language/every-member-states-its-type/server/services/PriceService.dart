import 'models/Price.dart';

@Service
class PriceService {
  static const String _currency = 'EUR';

  Price priceFor(int amount, int count) => Price(amount * count, _currency);
}
