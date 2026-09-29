using System.Text.Json.Nodes;
using Models;

[RequestRoute("/ledger")]
[Public]
public sealed class LedgerController {
    [Needs]
    public LedgerService Ledger;

    [GetRoute]
    public LedgerReport GetLedger() => BuildReport().GetAwaiter().GetResult();

    private async System.Threading.Tasks.Task<LedgerReport> BuildReport() {
        var settled = await Ledger.settle(BehaviorJSON.From("open"));
        var commands = new JsonArray();
        foreach (var command in LedgerAdapterHelper.commands) commands.Add(command);
        return new LedgerReport(JsonNode.Parse(settled.ToString()), commands);
    }
}
