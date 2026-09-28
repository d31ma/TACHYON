using System.Text.Json.Nodes;
using Models;

[RequestRoute("/ledger")]
[Public]
public sealed class LedgerController {
    [GetRoute]
    public LedgerReport GetLedger() => BuildReport().GetAwaiter().GetResult();

    private async System.Threading.Tasks.Task<LedgerReport> BuildReport() {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(LedgerSetup), "ledger", signal);
        var entries = new EntryStore(context);
        var settled = await entries.settle(BehaviorJSON.From("open"));
        var commands = new JsonArray();
        foreach (var command in LedgerAdapterHelper.commands) commands.Add(command);
        return new LedgerReport(JsonNode.Parse(settled.ToString()), commands);
    }
}
