open System.Threading.Tasks

[<Helper>]
type CustomHttpHelper() =
    interface BehaviorHTTPAdapter with
        member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
            execution.Check()
            Task.FromResult(BehaviorJSON.Parse("{\"message\":\"custom adapter\"}"))
