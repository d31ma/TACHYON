[<Delegate>]
type TaxDelegate() =
    [<Needs>]
    member val TaxService : TaxService = Unchecked.defaultof<TaxService> with get, set

    member this.Rate(region: string) : int =
        this.TaxService.Rate(region) * 2

    member this.Total(count: int, extra: int) : int =
        this.TaxService.Total(count) + extra
