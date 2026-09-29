[<Test(typeof<CatalogTestSetup>, {"partition":"tests"})>]
type CatalogServiceTest() =
    member this.Twice(value: int) : int =
        value * 2

    member this.testHasAContext() : unit =
        Assert.isTrue(not (isNull this.behaviors), "the case has a context")

    member this.testDoublesAValue() : unit =
        Assert.equals(8, this.Twice(4))
