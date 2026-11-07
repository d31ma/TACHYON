[<Setup>]
type OverrideHttpSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        execution.Check()
        let adapters: BehaviorAdapters = BehaviorAdapters()
        adapters.http.Add("probe", new CustomHttpDelegate())
        adapters
