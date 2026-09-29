type PublicValues() =
    [<Setting("${tac.catalog.title:Catalog}")>]
    member val title : string = "" with get, set
