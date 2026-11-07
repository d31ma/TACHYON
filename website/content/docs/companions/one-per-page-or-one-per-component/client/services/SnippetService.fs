[<Service>]
[<NeedsAll>]
type SnippetService(files: string list) =
    member this.Files : string list = files
