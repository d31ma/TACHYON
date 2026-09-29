[<Test(typeof<PriceTestSetup>)>]
type PriceUtilsTest() =
    member this.testAddsTheTax() : unit =
        Assert.equals(1200, PriceUtils.WithTax(1000L))

    [<ValueSource([0, 5, 1000])>]
    member this.testNeverLowersAPrice(cents: int64) : unit =
        Assert.isTrue(PriceUtils.WithTax(cents) >= cents, "the tax never lowers a price")

    member this.testRefusesANegativePrice() : unit =
        let error = Assert.throws(fun () -> PriceUtils.WithTax(-1L))
        Assert.equals("A price is never negative.", error.Message)
