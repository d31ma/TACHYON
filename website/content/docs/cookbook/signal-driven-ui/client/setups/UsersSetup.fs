[<Setup>]
type UsersSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        execution.Check()
        BehaviorAdapters()
