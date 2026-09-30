[<TestSetup>]
type GreetingTestSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        BehaviorAdapters()
