[Setup]
public sealed class PageProgramSetup
{
    sealed class ProgramTransport : BehaviorHTTPAdapter
    {
        private readonly string _identity = System.Guid.NewGuid().ToString();
        private int _count = 0;

        public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution)
        {
            execution.Check();
            if (request.Get("method").Text == "POST" && request.Get("path").Text.EndsWith("/increment"))
            {
                _count += 1;
            }
            var state = new System.Collections.Generic.Dictionary<string, BehaviorJSON>
            {
                ["identity"] = BehaviorJSON.From(_identity),
                ["count"] = BehaviorJSON.From(_count),
            };
            return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Object(state));
        }
    }

    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["pageprogram"] = new ProgramTransport();
        return adapters;
    }
}
