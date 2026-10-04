final class PriceUtils {
    static String label(int cents) {
        return "$" + dollars(cents);
    }

    private static String dollars(int cents) {
        return java.math.BigDecimal.valueOf(cents, 2).toPlainString();
    }
}
