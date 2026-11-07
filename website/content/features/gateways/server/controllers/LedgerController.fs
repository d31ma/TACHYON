open System.Text.Json.Nodes
open Models

[<RequestRoute("/ledger")>]
[<Public>]
type LedgerController(ledgerClient: LedgerClient) =
    [<GetRoute>]
    member this.GetLedger() : Balance =
        let balance = ledgerClient.Balance(BehaviorJSON.From("ACME-7"))
        let forecast = ledgerClient.Forecast(BehaviorJSON.From(1))
        { Balance = JsonNode.Parse(balance.ToString())
          Forecast = JsonNode.Parse(forecast.ToString()) }
