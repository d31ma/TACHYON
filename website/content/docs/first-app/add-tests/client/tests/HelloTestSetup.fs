[<TestSetup>]
type HelloTestSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        BehaviorAdapters()
