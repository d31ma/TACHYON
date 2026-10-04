open System.Threading.Tasks

[<Service>]
type CatalogService() =
    member val behaviors : BehaviorContext = Behaviors.create(typeof<ApplicationSetup>, "public-catalog", new BehaviorSignal())
    member val Offset : int64 = 0L with get, set

    member this.Page([<PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})>] input: BehaviorJSON) : Task<BehaviorJSON> =
        Task.FromResult(input.Get("offset"))

    [<OnStart>]
    member this.Initialize() : unit =
        this.Offset <- this.Page(BehaviorJSON.Parse("{\"page\":1}")).Result.Integer
