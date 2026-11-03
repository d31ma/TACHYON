[Setup]
public sealed class UsersSetup {
    sealed class UsersAdapter : BehaviorHTTPAdapter {
        private readonly List<BehaviorJSON> saved = new List<BehaviorJSON>();

        public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution) {
            if (request.Get("method").Text == "GET") return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Array(saved.ToArray()));
            var user = BehaviorJSON.Object(new Dictionary<string, BehaviorJSON> { ["name"] = request.Get("body") });
            saved.Add(user);
            return System.Threading.Tasks.Task.FromResult(user);
        }
    }

    public static BehaviorAdapters configure(BehaviorExecution execution) {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["users"] = new UsersAdapter();
        return adapters;
    }
}
