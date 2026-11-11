[<Setup>]
type OrderSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        execution.Check()
        BehaviorAdapters()
