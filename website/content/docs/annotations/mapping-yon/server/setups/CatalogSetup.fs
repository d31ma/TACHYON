[<Setup>]
type CatalogSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters = BehaviorAdapters()
