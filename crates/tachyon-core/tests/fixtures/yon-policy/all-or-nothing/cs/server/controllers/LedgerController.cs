using System.Text.Json.Nodes;
using Models;

[RequestRoute("/ledger")]
[Public]
public sealed class LedgerController {
    private const string Open = "open";

    [Needs]
    public LedgerService Ledger;

    [GetRoute]
    public LedgerReport GetLedger() => BuildReport().GetAwaiter().GetResult();

    private async System.Threading.Tasks.Task<LedgerReport> BuildReport() {
        var settled = await Ledger.settle(BehaviorJSON.From(Open));
        await Refuse();
        var commands = new JsonArray();
        foreach (var command in LedgerAdapterDelegate.commands) commands.Add(command);
        return new LedgerReport(JsonNode.Parse(settled.ToString()), commands);
    }

    private async System.Threading.Tasks.Task Refuse() {
        try {
            await Ledger.refuse(BehaviorJSON.From(Open));
        } catch (System.InvalidOperationException refused) {
            LedgerAdapterDelegate.commands.Add(refused.Message);
        }
    }
}
