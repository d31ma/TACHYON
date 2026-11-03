[<Service>]
type TaxService() =
    member this.Rate(region: string) : int =
        2

    member this.Total(count: int) : int =
        count * 10
