struct PriceUtils {
    static func label(_ cents: Int) -> String {
        "$" + dollars(cents)
    }

    private static func dollars(_ cents: Int) -> String {
        let (whole, rest) = cents.quotientAndRemainder(dividingBy: 100)
        let fraction = String(rest)
        return "\(whole)." + (fraction.count == 1 ? "0" : "") + fraction
    }
}
