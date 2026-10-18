open System.Threading.Tasks

[<Setup>]
type OverrideSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        execution.Check()
        let adapters: BehaviorAdapters = BehaviorAdapters()
        let answer: BehaviorHTTPAdapter =
            { new BehaviorHTTPAdapter with
                member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
                    Task.FromResult(BehaviorJSON.Parse("{\"message\":\"Custom adapter wins\"}")) }
        adapters.http.Add("override", answer)
        adapters
