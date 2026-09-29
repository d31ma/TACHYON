open System
open System.Threading.Tasks

[<Service>]
type OrdersService() =
    let firstWorkingAttempt = 3
    let mutable attempts = 0

    member val behaviors : BehaviorContext = Behaviors.create(typeof<OrdersSetup>, "orders", new BehaviorSignal())

    [<Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})>]
    member this.submit(id: BehaviorJSON) : Task<BehaviorJSON> =
        attempts <- attempts + 1
        this.FailFirstTwoAttempts()
        Task.FromResult(this.ReportOf(id))

    [<MaxAtOnce({"limit":2,"policy":"block"})>]
    member this.report(id: BehaviorJSON) : Task<BehaviorJSON> =
        Task.FromResult(this.ReportOf(id))

    member private this.ReportOf(id: BehaviorJSON) : BehaviorJSON =
        BehaviorJSON.Object(dict [ "id", id; "attempts", BehaviorJSON.From(attempts) ])

    member private this.FailFirstTwoAttempts() : unit =
        if attempts < firstWorkingAttempt then
            raise (InvalidOperationException("the payment gateway is unavailable"))
