[<Checked>]
type OrderInput() =
    [<NotEmpty>]
    [<Length({"min":1,"max":80})>]
    member val name : string = "" with get, set

    [<Required>]
    [<Min(1)>]
    [<Max(9)>]
    member val level : float = 0.0 with get, set
