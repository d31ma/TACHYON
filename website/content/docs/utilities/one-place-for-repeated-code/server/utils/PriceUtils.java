final class PriceUtils {
    static String label(long cents) {
        return "$" + dollars(cents);
    }

    private static String dollars(long cents) {
        return java.math.BigDecimal.valueOf(cents, 2).toPlainString();
    }
}
