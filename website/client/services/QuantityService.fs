[<Service>]
type QuantityService() =
    let one = 1
    let space = " "
    let plural = "s"

    member this.Describe(count: int, noun: string) : string =
        count.ToString() + space + this.NounFor(count, noun)

    member private this.NounFor(count: int, noun: string) : string =
        if count = one then noun else noun + plural
