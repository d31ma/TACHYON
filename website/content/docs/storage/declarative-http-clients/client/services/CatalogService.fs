open System.Threading.Tasks

[<Service>]
type CatalogService() =
    let catalog = CatalogClient()
    member val products : string list = [] with get, set
    member val error : string = "" with get, set

    member this.Load() : Task =
        task {
            let! names = this.Fetch()
            this.Show(names)
        }

    member private this.Fetch() : Task<string list option> =
        task {
            try
                let! answer = catalog.All()
                return Some(answer.Items |> Array.map (fun item -> item.Text) |> List.ofArray)
            with _ ->
                return None
        }

    member private this.Show(names: string list option) : unit =
        match names with
        | Some list ->
            this.products <- list
            this.error <- ""
        | None -> this.ShowFailure()

    member private this.ShowFailure() : unit =
        this.error <- "Products could not be loaded. Please retry."
