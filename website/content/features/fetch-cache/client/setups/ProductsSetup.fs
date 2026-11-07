open System.Threading.Tasks

[<Setup>]
type ProductsSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        let adapters = BehaviorAdapters()
        adapters.http["products"] <-
            { new BehaviorHTTPAdapter with
                member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
                    Task.FromResult(BehaviorJSON.Parse("{\"items\":[\"anvil\",\"hammer\"]}")) }
        adapters
