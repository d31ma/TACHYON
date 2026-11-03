open System.Threading.Tasks

[<Delegate>]
type GreetingFakeDelegate() =
    interface BehaviorHTTPAdapter with
        member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
            execution.Check()
            Task.FromResult(BehaviorJSON.Parse("{\"message\":\"Hi\"}"))
