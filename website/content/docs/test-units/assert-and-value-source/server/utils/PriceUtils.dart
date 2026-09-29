class PriceUtils {
  static int withTax(int cents) {
    _requirePositive(cents);
    return (cents * 1.2).round();
  }

  static void _requirePositive(int cents) {
    if (cents < 0) {
      throw ArgumentError('A price is never negative.');
    }
  }
}
