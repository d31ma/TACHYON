using System.Text.Json.Nodes;
using Models;

[RequestRoute("/ledger")]
[Public]
class LedgerController {
    [Needs]
    readonly LedgerClient _ledgerClient;

    [GetRoute]
    public Ledger GetLedger() {
        var balance = _ledgerClient.Balance(BehaviorJSON.From("ACME-7"));
        var forecast = _ledgerClient.Forecast(BehaviorJSON.From(1));
        return new Ledger(JsonNode.Parse(balance.ToString()), JsonNode.Parse(forecast.ToString()));
    }
}
