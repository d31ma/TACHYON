final class PriceUtils {
    static String label(long cents) {
        return '$' + dollars(cents)
    }

    private static String dollars(long cents) {
        return BigDecimal.valueOf(cents, 2).toPlainString()
    }
}
