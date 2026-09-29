open System.Threading.Tasks

[<Helper>]
type CatalogTransportHelper() =
    interface BehaviorHTTPAdapter with
        member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
            Task.FromResult(BehaviorJSON.Parse("{\"items\":[\"anvil\"]}"))
