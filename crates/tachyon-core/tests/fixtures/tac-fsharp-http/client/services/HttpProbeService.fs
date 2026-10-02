open System.Threading.Tasks

[<Service>]
[<Lifetime({"value":"prototype"})>]
[<Description("Ordinary F# browser Task HTTP owner and transport conformance")>]
type HttpProbeService() =
    let signal: BehaviorSignal = new BehaviorSignal()
    let context: BehaviorContext = Behaviors.create(typeof<BrowserHttpSetup>, "fsharp-http", signal)
    let customContext: BehaviorContext = Behaviors.create(typeof<OverrideHttpSetup>, "fsharp-custom", signal)

    member val Last: string = "ready" with get, set
    member val OverrideLast: string = "ready" with get, set

    member this.Read() : Task<BehaviorJSON> =
        task {
            let! answer = context.restClient("probe").get("/api/probe").retrieve()
            this.Last <- answer.Get("message").Text
            return answer
        }

    member this.Slow() : Task<BehaviorJSON> =
        context.restClient("probe").get("/api/slow").retrieve()

    member this.InvalidPath() : Task<BehaviorJSON> =
        context.restClient("probe").get("//untrusted.example").retrieve()

    member this.Custom() : Task<BehaviorJSON> =
        task {
            let! answer = customContext.restClient("probe").get("/api/probe").retrieve()
            this.OverrideLast <- answer.Get("message").Text
            return answer
        }

    [<OnStop>]
    member this.Close() : unit =
        context.Dispose()
        customContext.Dispose()
        signal.Dispose()
