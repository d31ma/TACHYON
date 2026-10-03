[Helper]
public sealed class CollectionAnswerHelper : BehaviorHTTPAdapter
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
            "/api/collections/empty" => "{\"items\":[]}",
            "/api/collections/nullable" => "{\"items\":[],\"optionalItems\":[null,{\"message\":\"nullable item\"}]}",
            "/api/collections/missing" => "{}",
            "/api/collections/wrong-array" => "{\"items\":{}}",
            "/api/collections/missing-member" => "{\"items\":[{}]}",
            "/api/collections/wrong-member" => "{\"items\":[{\"message\":7}]}",
            "/api/collections/extra-member" => "{\"items\":[{\"message\":\"first\",\"extra\":true}]}",
            "/api/collections/null-member" => "{\"items\":[null]}",
            _ => "{\"items\":[{\"message\":\"first\"},{\"message\":\"second\"}]}"
        };
        return BehaviorJSON.Parse(json);
    }
}
