[Service]
class CatalogService {
    [Needs]
    readonly CatalogClient _catalog;

    public object Products() {
        BehaviorJSON answer = _catalog.All(BehaviorJSON.From(1)).GetAwaiter().GetResult();
        return new System.Collections.Generic.Dictionary<string, object> {
            ["items"] = TextsOf(answer.Get("items").Items),
        };
    }

    private static string[] TextsOf(BehaviorJSON[] items) {
        return System.Array.ConvertAll(items, item => item.Text);
    }
}
