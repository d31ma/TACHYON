[<Test(typeof<HelloTestSetup>, {"partition":"tests"})>]
type CounterServiceTest() =
    [<Fake>]
    member val GreetingClient : GreetingClient = Unchecked.defaultof<GreetingClient> with get, set

    member this.testIncrementAddsOne() : unit =
        let counter: CounterService = this.construct<CounterService>()
        counter.Increment()
        Assert.equals(1, counter.Count)

    member this.testAskServerSavesTheTypedMessage() : unit =
        let counter: CounterService = this.construct<CounterService>()
        let greeting = counter.AskServer().GetAwaiter().GetResult()
        Assert.equals("Hi", greeting.Message)
        Assert.equals("Hi", counter.Message)

    member this.testConstructedCountersKeepTheirOwnClientContext() : unit =
        use firstSignal = new BehaviorSignal()
        use secondSignal = new BehaviorSignal()
        use firstContext = Behaviors.create(typeof<HelloTestSetup>, "first-counter", firstSignal)
        use secondContext = Behaviors.create(typeof<HelloTestSetup>, "second-counter", secondSignal)
        this.GreetingClient <- GreetingClient(firstContext)
        let first: CounterService = this.construct<CounterService>()
        first.Increment()
        this.GreetingClient <- GreetingClient(secondContext)
        let second: CounterService = this.construct<CounterService>()
        this.GreetingClient <- Unchecked.defaultof<GreetingClient>
        firstContext.Dispose()
        let refusal = this.RefusalOf(first)
        Assert.equals("CANCELLED", refusal)
        Assert.equals("", first.Message)
        Assert.equals("Hi", second.AskServer().GetAwaiter().GetResult().Message)
        Assert.equals("Hi", second.Message)
        Assert.equals(1, first.Count)
        Assert.equals(0, second.Count)

    member private this.RefusalOf(counter: CounterService) : string =
        try
            counter.AskServer().GetAwaiter().GetResult() |> ignore
            ""
        with :? BehaviorError as error ->
            error.Code
