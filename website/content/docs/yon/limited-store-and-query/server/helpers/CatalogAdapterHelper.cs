[Helper]
public sealed class CatalogAdapterHelper : BehaviorEntityAdapter {
    static readonly BehaviorJSON Rows = BehaviorJSON.Parse("[{\"id\":1,\"status\":\"active\"},{\"id\":2,\"status\":\"retired\"}]");

    public System.Threading.Tasks.Task<BehaviorJSON> execute(BehaviorJSON command, BehaviorExecution execution) {
        var query = command.Get("query");
        var condition = query.Get("where").Items[0];
        bool keepUnequalRows = condition.Get("operator").Text == "ne";
        var matchingRows = new System.Collections.Generic.List<BehaviorJSON>();
        foreach (var row in Rows.Items) {
            bool isEqual = row.Get(condition.Get("field").Text).ToString() == condition.Get("value").ToString();
            bool isMatch = keepUnequalRows ? !isEqual : isEqual;
            if (isMatch) matchingRows.Add(row);
        }
        if (query.Get("kind").Text == "count") {
            return System.Threading.Tasks.Task.FromResult(BehaviorJSON.From(matchingRows.Count));
        }
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Array(matchingRows.ToArray()));
    }
}
