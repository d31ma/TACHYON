[<Service>]
type EditorService() =
    [<SessionState>]
    member val Draft : string = "" with get, set

    [<LocalState>]
    member val Theme : string = "light" with get, set
