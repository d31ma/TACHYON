[<Setup>]
type CatalogSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        let adapters = BehaviorAdapters()
        adapters.http["catalog"] <- CatalogTransportDelegate()
        adapters
