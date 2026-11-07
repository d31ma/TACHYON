[Delegate]
public sealed class LedgerAdapterDelegate : BehaviorEntityAdapter {
    private const string Transaction = "transaction";
    private const string Operation = "operation";
    public static readonly System.Collections.Generic.List<string> commands = new System.Collections.Generic.List<string>();
    private static int sequence = 0;

    public System.Threading.Tasks.Task<BehaviorJSON> execute(BehaviorJSON command, BehaviorExecution execution) {
        var identifier = command.Has(Transaction) ? command.Get(Transaction).Text : null;
        if (command.Get(Operation).Text == Transaction) {
            var action = command.Get("action").Text;
            var entry = action;
            if (identifier != null) entry = action + ":" + identifier;
            commands.Add(entry);
            if (action != "begin") return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"ok\":true}"));
            sequence++;
            var answer = BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> {
                ["transaction"] = BehaviorJSON.From("t" + sequence),
            });
            return System.Threading.Tasks.Task.FromResult(answer);
        }
        if (command.Get(Operation).Text == "save") return Saved(command, identifier);
        var kind = command.Get("query").Get("kind").Text;
        var transaction = identifier ?? "none";
        commands.Add(kind + "@" + transaction);
        if (kind == "count") return System.Threading.Tasks.Task.FromResult(BehaviorJSON.From(1));
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("[{\"id\":1,\"status\":\"open\"}]"));
    }

    private static System.Threading.Tasks.Task<BehaviorJSON> Saved(BehaviorJSON command, string identifier) {
        commands.Add("save@" + identifier);
        return System.Threading.Tasks.Task.FromResult(command);
    }
}
