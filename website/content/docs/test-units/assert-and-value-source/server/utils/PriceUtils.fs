type PriceUtils =
    static member WithTax(cents: int64) : int64 =
        PriceUtils.RequirePositive(cents)
        int64 (System.Math.Round(decimal cents * 1.2m))

    static member private RequirePositive(cents: int64) : unit =
        if cents < 0L then raise (System.ArgumentException("A price is never negative."))
