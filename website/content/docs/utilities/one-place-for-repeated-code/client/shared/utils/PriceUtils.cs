static class PriceUtils {
    public static string Label(int cents) {
        return "$" + Dollars(cents);
    }

    private static string Dollars(int cents) {
        return (cents / 100m).ToString("0.00", System.Globalization.CultureInfo.InvariantCulture);
    }
}
