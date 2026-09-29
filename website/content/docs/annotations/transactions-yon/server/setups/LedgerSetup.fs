open System
open System.Threading.Tasks

[<Setup>]
type LedgerSetup() =
    static member configure(execution: BehaviorExecution) : BehaviorAdapters =
        let adapters = BehaviorAdapters()
        adapters.entities["ledger"] <- LedgerAdapterHelper()
        adapters.authorizeEntity <- Func<_, _, _>(fun command current -> Task.FromResult(true))
        adapters
