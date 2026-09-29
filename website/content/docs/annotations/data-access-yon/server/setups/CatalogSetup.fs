open System
open System.Threading.Tasks

[<Setup>]
type CatalogSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        let adapters = BehaviorAdapters()
        adapters.entities["catalog"] <- CatalogAdapterHelper()
        adapters.authorizeEntity <- Func<_, _, _>(fun command current -> Task.FromResult(command.Get("entity").Text = "ProductEntity"))
        adapters
