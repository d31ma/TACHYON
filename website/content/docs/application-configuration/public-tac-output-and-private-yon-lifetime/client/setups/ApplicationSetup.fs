[<Setup>]
type ApplicationSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        execution.Check()
        BehaviorAdapters()
