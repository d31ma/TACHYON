[<Test(typeof<PriceTestSetup>, {"partition":"tests"})>]
type PriceUtilsTest() =
    member this.testLabelShowsDollars() : unit =
        Assert.equals("$12.50", PriceUtils.Label(1250L))
