type PriceUtils =
    static member Label(cents: int) : string =
        sprintf "$%s" (PriceUtils.Dollars(cents))

    static member private Dollars(cents: int) : string =
        (decimal cents / 100m).ToString("0.00", System.Globalization.CultureInfo.InvariantCulture)
