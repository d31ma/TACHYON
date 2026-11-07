open System.Collections.Generic
open System.Threading.Tasks

[<Setup>]
type UsersSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        execution.Check()
        let saved = List<BehaviorJSON>()
        let adapters = BehaviorAdapters()
        adapters.http["users"] <-
            { new BehaviorHTTPAdapter with
                member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
                    if request.Get("method").Text = "GET" then
                        Task.FromResult(BehaviorJSON.Array(saved.ToArray()))
                    else
                        let user = BehaviorJSON.Object(dict [ "name", request.Get("body") ])
                        saved.Add(user)
                        Task.FromResult(user) }
        adapters
