static class PriceUtils {
    public static long WithTax(long cents) {
        RequirePositive(cents);
        return (long)Math.Round(cents * 1.2m);
    }

    static void RequirePositive(long cents) {
        if (cents < 0) throw new ArgumentException("A price is never negative.");
    }
}
