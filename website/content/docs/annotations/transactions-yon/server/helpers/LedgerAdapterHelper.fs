open System.Collections.Generic
open System.Threading.Tasks

[<Helper>]
type LedgerAdapterHelper() =
    let transactionKey = "transaction"

    static member val commands = List<string>() with get
    static member val sequence = 0 with get, set

    interface BehaviorEntityAdapter with
        member this.execute(command: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
            let identifier = if command.Has(transactionKey) then command.Get(transactionKey).Text else null
            if command.Get("operation").Text = transactionKey then
                this.Transaction(command.Get("action").Text, identifier)
            else
                this.Query(command.Get("query").Get("kind").Text, identifier)

    member private this.Transaction(action: string, identifier: string) : Task<BehaviorJSON> =
        LedgerAdapterHelper.commands.Add(if isNull identifier then action else action + ":" + identifier)
        if action <> "begin" then
            Task.FromResult(BehaviorJSON.Parse("{\"ok\":true}"))
        else
            LedgerAdapterHelper.sequence <- LedgerAdapterHelper.sequence + 1
            let started = "t" + string LedgerAdapterHelper.sequence
            Task.FromResult(BehaviorJSON.Object(dict [ transactionKey, BehaviorJSON.From(started) ]))

    member private this.Query(kind: string, identifier: string) : Task<BehaviorJSON> =
        let transaction = if isNull identifier then "none" else identifier
        LedgerAdapterHelper.commands.Add(kind + "@" + transaction)
        if kind = "count" then
            Task.FromResult(BehaviorJSON.From(1))
        else
            Task.FromResult(BehaviorJSON.Parse("[{\"id\":1,\"status\":\"open\"}]"))
