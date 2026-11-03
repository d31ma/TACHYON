[Setup]
public sealed class BillingSetup {
    public sealed class BillingAdapter : BehaviorEntityAdapter {
        public System.Threading.Tasks.Task<BehaviorJSON> execute(BehaviorJSON command, BehaviorExecution execution) {
            bool isCountQuery = command.Get("query").Get("kind").Text == "count";
            if (isCountQuery) return System.Threading.Tasks.Task.FromResult(BehaviorJSON.From(0));
            return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("[]"));
        }
    }

    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        adapters.entities["publicBilling"] = new BillingAdapter();
        adapters.authorizeEntity = (command, current) =>
            System.Threading.Tasks.Task.FromResult(command.Get("entity").Text == "BillingEntity");
        return adapters;
    }
}
