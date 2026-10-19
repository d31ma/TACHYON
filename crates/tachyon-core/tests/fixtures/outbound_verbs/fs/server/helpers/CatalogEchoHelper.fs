open System.Threading.Tasks

[<Helper>]
type CatalogEchoHelper() =
    static member private Sent(request: BehaviorJSON, key: string) : BehaviorJSON =
        if request.Has(key) then request.Get(key) else BehaviorJSON.Parse("null")

    interface BehaviorHTTPAdapter with
        member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
            let sent = System.Collections.Generic.Dictionary<string, BehaviorJSON>()
            for key in [ "method"; "path"; "query"; "headers"; "body" ] do
                sent[key] <- CatalogEchoHelper.Sent(request, key)
            Task.FromResult(BehaviorJSON.Object(sent))
