[Helper]
public sealed class ResultAnswerHelper : BehaviorHTTPAdapter
{
    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution)
    {
        execution.Check();
        return System.Threading.Tasks.Task.FromResult(_response(request.Get("path").Text));
    }

    private static BehaviorJSON _response(string path)
    {
        var json = path switch
        {
            "/api/results/items" => "[{\"name\":\"fake first\"},{\"name\":\"fake second\"}]",
            "/api/results/array" => "[{\"name\":\"fake array\"}]",
            "/api/results/count" => "7",
            "/api/results/label" => "\"fake label\"",
            "/api/results/ready" => "true",
            "/api/results/ratio" => "0.5",
            "/api/results/numbers" => "[3,1,2]",
            "/api/results/shelf" => "{\"label\":\"fake shelf\",\"detail\":{\"note\":\"fake note\",\"count\":2}}",
            "/api/results/first" => "{\"name\":\"fake value task\"}",
            "/api/results/wrong-items" => "{\"name\":\"not a list\"}",
            "/api/results/wrong-count" => "\"seven\"",
            "/api/results/wrong-shelf" => "{\"label\":\"broken shelf\",\"detail\":{\"note\":\"no count\"}}",
            _ => "null"
        };
        return BehaviorJSON.Parse(json);
    }
}
