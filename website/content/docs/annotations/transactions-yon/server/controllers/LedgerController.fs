open System.Text.Json.Nodes
open System.Threading.Tasks
open Models

[<RequestRoute("/ledger")>]
[<Public>]
type LedgerController() =
    [<GetRoute>]
    member this.GetLedger() : LedgerReport = this.BuildReport().Result

    member private this.BuildReport() : Task<LedgerReport> =
        task {
            use signal = new BehaviorSignal()
            use context = Behaviors.create(typeof<LedgerSetup>, "ledger", signal)
            let entries = EntryStore(context)
            let! settled = entries.settle(BehaviorJSON.From("open"))
            let commands = JsonArray()
            for command in LedgerAdapterHelper.commands do
                commands.Add(JsonValue.Create(command))
            return { Settled = JsonNode.Parse(settled.ToString()); Commands = commands :> JsonNode }
        }
