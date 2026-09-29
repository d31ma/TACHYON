[<Test(typeof<CatalogTestSetup>)>]
type CatalogServiceTest() =
    member this.Twice(value: int) : int =
        value * 2

    member this.testDoublesAValue() : unit =
        Assert.equals(8, this.Twice(4))
