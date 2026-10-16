open System.Threading.Tasks

[<Helper>]
type GreetingFakeHelper() =
    interface BehaviorHTTPAdapter with
        member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
            execution.Check()
            Task.FromResult(BehaviorJSON.Parse("{\"message\":\"Hi\"}"))
