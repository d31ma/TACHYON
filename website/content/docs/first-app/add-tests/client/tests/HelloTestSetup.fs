[<TestSetup>]
type HelloTestSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        execution.Check()
        let adapters: BehaviorAdapters = BehaviorAdapters()
        adapters.http.Add("greeting", new GreetingFakeDelegate())
        adapters
