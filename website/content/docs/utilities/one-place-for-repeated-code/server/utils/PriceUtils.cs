static class PriceUtils {
    public static string Label(long cents) {
        return "$" + Dollars(cents);
    }

    private static string Dollars(long cents) {
        return (cents / 100m).ToString("0.00", System.Globalization.CultureInfo.InvariantCulture);
    }
}
