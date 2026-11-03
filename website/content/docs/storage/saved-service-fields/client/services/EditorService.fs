[<Service>]
type EditorService() =
    [<SessionState>]
    member val Draft = "" with get, set

    [<LocalState>]
    member val Theme = "calm" with get, set
