final class PriceUtils {
    static long withTax(long cents) {
        requirePositive(cents);
        return Math.round(cents * 1.2);
    }

    private static void requirePositive(long cents) {
        if (cents < 0) throw new IllegalArgumentException("A price is never negative.");
    }
}
