open System.Threading.Tasks

[<Service>]
[<Lifetime({"value":"prototype"})>]
[<Description("Ordinary F# browser Task HTTP owner and transport conformance")>]
type HttpProbeService() =
    static let probeName: string = "probe"
    static let probePath: string = "/api/probe"
    static let messageField: string = "message"

    let signal: BehaviorSignal = new BehaviorSignal()
    let context: BehaviorContext = Behaviors.create(typeof<BrowserHttpSetup>, "fsharp-http", signal)
    let customContext: BehaviorContext = Behaviors.create(typeof<OverrideHttpSetup>, "fsharp-custom", signal)

    member val Last: string = "ready" with get, set
    member val OverrideLast: string = "ready" with get, set

    member this.Read() : Task<BehaviorJSON> =
        task {
            let! answer = context.restClient(probeName).get(probePath).retrieve()
            this.Last <- answer.Get(messageField).Text
            return answer
        }

    member this.Slow() : Task<BehaviorJSON> =
        context.restClient(probeName).get("/api/slow").retrieve()

    member this.InvalidPath() : Task<BehaviorJSON> =
        context.restClient(probeName).get("//untrusted.example").retrieve()

    member this.Custom() : Task<BehaviorJSON> =
        task {
            let! answer = customContext.restClient(probeName).get(probePath).retrieve()
            this.OverrideLast <- answer.Get(messageField).Text
            return answer
        }

    [<OnStop>]
    member this.Close() : unit =
        context.Dispose()
        customContext.Dispose()
        signal.Dispose()
