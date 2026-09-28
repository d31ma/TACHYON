[<Test(typeof<CatalogTestSetup>)>]
type CatalogServiceTest() =
    member this.Twice(value: int) : int =
        value * 2

    member this.testDoublesAValue() : unit =
        if this.Twice(4) <> 8 then failwith "expected 8"
