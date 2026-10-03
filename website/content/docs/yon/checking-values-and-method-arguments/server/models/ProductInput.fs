[<Checked>]
type ProductInput() =
    [<NotEmpty>]
    [<Length({"min":1,"max":80})>]
    member val name : BehaviorJSON = BehaviorJSON.Parse("null") with get, set

    [<Required>]
    [<Min(1)>]
    [<Max(9)>]
    member val level : BehaviorJSON = BehaviorJSON.Parse("null") with get, set
