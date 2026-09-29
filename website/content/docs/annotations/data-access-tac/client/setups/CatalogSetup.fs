open System
open System.Threading.Tasks

[<Setup>]
type CatalogSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        let adapters = BehaviorAdapters()
        adapters.entities["publicCatalog"] <-
            { new BehaviorEntityAdapter with
                member this.execute(command: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
                    let isCountQuery = command.Get("query").Get("kind").Text = "count"
                    if isCountQuery then
                        Task.FromResult(BehaviorJSON.From(0))
                    else
                        Task.FromResult(BehaviorJSON.Parse("[]")) }
        adapters.authorizeEntity <- Func<_, _, _>(fun command current -> Task.FromResult(command.Get("entity").Text = "ProductEntity"))
        adapters
