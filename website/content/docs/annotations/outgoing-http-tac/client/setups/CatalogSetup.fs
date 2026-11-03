open System.Threading.Tasks

[<Setup>]
type CatalogSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        let adapters = BehaviorAdapters()
        adapters.http["catalog"] <-
            { new BehaviorHTTPAdapter with
                member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
                    Task.FromResult(BehaviorJSON.Parse("{\"items\":[\"anvil\"]}")) }
        adapters
