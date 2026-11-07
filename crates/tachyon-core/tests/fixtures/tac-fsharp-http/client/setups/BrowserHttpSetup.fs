[<Setup>]
type BrowserHttpSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        execution.Check()
        BehaviorAdapters()
