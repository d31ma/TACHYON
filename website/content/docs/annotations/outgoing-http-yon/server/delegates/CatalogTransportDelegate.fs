open System.Threading.Tasks

[<Delegate>]
type CatalogTransportDelegate() =
    interface BehaviorHTTPAdapter with
        member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
            Task.FromResult(BehaviorJSON.Parse("{\"items\":[\"anvil\"]}"))
