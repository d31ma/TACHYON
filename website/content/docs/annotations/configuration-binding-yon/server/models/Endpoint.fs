type Endpoint() =
    [<Setting("${yon.active.profile:development}")>]
    member val profile : string = "" with get, set
