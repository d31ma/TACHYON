type PublicSettings() =
    [<Setting("${tac.api.url:/api}")>]
    member val api : string = "" with get, set
