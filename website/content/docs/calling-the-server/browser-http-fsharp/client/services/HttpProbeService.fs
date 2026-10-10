open System.Threading.Tasks

[<Service>]
[<Lifetime({"value":"prototype"})>]
type HttpProbeService() =
    static let clientName: string = "greeting"
    static let requestPath: string = "/api/greeting"
    static let messageField: string = "message"

    let signal: BehaviorSignal = new BehaviorSignal()
    let context: BehaviorContext = Behaviors.create(typeof<BrowserHttpSetup>, "fsharp-greeting", signal)

    member val Last: string = "" with get, set

    member this.Read() : Task<BehaviorJSON> =
        task {
            let! answer = context.restClient(clientName).get(requestPath).retrieve()
            this.Last <- answer.Get(messageField).Text
            return answer
        }

    [<OnStop>]
    member this.Close() : unit =
        context.Dispose()
        signal.Dispose()
