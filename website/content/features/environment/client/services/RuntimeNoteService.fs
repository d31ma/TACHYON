[<Service>]
type RuntimeNoteService() =
    member val Runtime : string = "F#, compiled by .NET" with get, set

    member this.ShowsNotes(development: bool) : bool =
        development
