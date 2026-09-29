[<Service>]
type QuantityService() =
    let one = 1

    member this.Describe(count: int, noun: string) : string =
        $"{count} {this.NounFor(count, noun)}"

    member private this.NounFor(count: int, noun: string) : string =
        if count = one then noun else noun + "s"
