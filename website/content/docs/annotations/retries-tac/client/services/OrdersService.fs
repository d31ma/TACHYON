open System
open System.Threading.Tasks

type OrdersService() =
    let firstWorkingAttempt = 3

    member val behaviors : BehaviorContext = Behaviors.create(typeof<OrdersSetup>, "orders", new BehaviorSignal())
    member val attempts : int = 0 with get, set

    [<Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})>]
    member this.submit(id: BehaviorJSON) : Task<BehaviorJSON> =
        this.attempts <- this.attempts + 1
        this.FailFirstTwoAttempts()
        Task.FromResult(this.ReportOf(id))

    [<MaxAtOnce({"limit":2,"policy":"block"})>]
    member this.report(id: BehaviorJSON) : Task<BehaviorJSON> =
        Task.FromResult(this.ReportOf(id))

    member private this.ReportOf(id: BehaviorJSON) : BehaviorJSON =
        BehaviorJSON.Object(dict [ "id", id; "attempts", BehaviorJSON.From(this.attempts) ])

    member private this.FailFirstTwoAttempts() : unit =
        if this.attempts < firstWorkingAttempt then
            raise (InvalidOperationException("the checkout API is unavailable"))
