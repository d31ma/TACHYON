open System.Collections.Generic
open System.Threading.Tasks

[<Setup>]
type PageProgramSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        execution.Check()
        let identity = System.Guid.NewGuid().ToString()
        let count = ref 0
        let adapters = BehaviorAdapters()
        adapters.http["pageprogram"] <-
            { new BehaviorHTTPAdapter with
                member this.exchange(request: BehaviorJSON, running: BehaviorExecution) : Task<BehaviorJSON> =
                    running.Check()
                    if request.Get("method").Text = "POST" && request.Get("path").Text.EndsWith("/increment") then
                        count.Value <- count.Value + 1
                    let state = Dictionary<string, BehaviorJSON>()
                    state["identity"] <- BehaviorJSON.From(identity)
                    state["count"] <- BehaviorJSON.From(count.Value)
                    Task.FromResult(BehaviorJSON.Object(state)) }
        adapters
