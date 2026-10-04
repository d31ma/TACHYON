class PriceUtils {
  static String label(int cents) {
    return '\$${_dollars(cents)}';
  }

  static String _dollars(int cents) {
    return (cents / 100).toStringAsFixed(2);
  }
}
