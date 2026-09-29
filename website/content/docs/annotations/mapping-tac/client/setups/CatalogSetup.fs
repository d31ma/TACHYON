open System
open System.Threading.Tasks

[<Setup>]
type CatalogSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        let adapters = BehaviorAdapters()
        adapters.authorize <- Func<_, _, _>(fun call current -> Task.FromResult(true))
        adapters
