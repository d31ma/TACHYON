[<Setup>]
type GreetingSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        execution.Check()
        BehaviorAdapters()
