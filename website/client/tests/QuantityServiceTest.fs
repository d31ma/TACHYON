[<Test(typeof<SiteTestSetup>)>]
type QuantityServiceTest() =
    member this.testOneThingIsSingular() : unit =
        Assert.equals("1 feature", QuantityService().Describe(1, "feature"), "one feature is singular")

    member this.testOtherCountsArePlural() : unit =
        Assert.equals("12 features", QuantityService().Describe(12, "feature"), "twelve features are plural")
        Assert.equals("0 guides", QuantityService().Describe(0, "guide"), "no guides are plural")
