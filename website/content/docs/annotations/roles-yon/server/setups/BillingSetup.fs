[<Setup>]
type BillingSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        BehaviorAdapters()
