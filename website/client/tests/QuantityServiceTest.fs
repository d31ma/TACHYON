[<Test(typeof<SiteTestSetup>)>]
type QuantityServiceTest() =
    member this.testOneThingIsSingular() : unit =
        if QuantityService().Describe(1, "feature") <> "1 feature" then failwith "one feature is singular"

    member this.testOtherCountsArePlural() : unit =
        if QuantityService().Describe(12, "feature") <> "12 features" then failwith "twelve features are plural"
        if QuantityService().Describe(0, "guide") <> "0 guides" then failwith "no guides are plural"
