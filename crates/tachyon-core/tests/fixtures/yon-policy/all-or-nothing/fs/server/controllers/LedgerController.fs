open System.Text.Json.Nodes
open System.Threading.Tasks
open Models

[<RequestRoute("/ledger")>]
[<Public>]
type LedgerController(ledger: LedgerService) =
    static let opened = "open"

    [<GetRoute>]
    member this.GetLedger() : LedgerReport = this.BuildReport().Result

    member private this.BuildReport() : Task<LedgerReport> =
        task {
            let! settled = ledger.settle(BehaviorJSON.From(opened))
            do! this.Refuse()
            let commands = JsonArray()
            for command in LedgerAdapterDelegate.commands do
                commands.Add(JsonValue.Create(command))
            return { Settled = JsonNode.Parse(settled.ToString()); Commands = commands :> JsonNode }
        }

    member private this.Refuse() : Task =
        task {
            try
                let! _ = ledger.refuse(BehaviorJSON.From(opened))
                ()
            with :? System.InvalidOperationException as refused ->
                LedgerAdapterDelegate.commands.Add(refused.Message)
        }
