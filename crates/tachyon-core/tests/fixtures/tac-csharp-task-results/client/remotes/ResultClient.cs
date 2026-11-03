[HTTPClient({"path":"/api/results","adapter":"results","setup":"ResultSetup"})]
public interface ResultClient
{
    [GetRequest("/items")]
    System.Threading.Tasks.Task<List<Models.ShelfItem>> items();

    [GetRequest("/array")]
    System.Threading.Tasks.Task<Models.ShelfItem[]> array();

    [GetRequest("/count")]
    System.Threading.Tasks.Task<int> count();

    [GetRequest("/label")]
    System.Threading.Tasks.Task<string> label();

    [GetRequest("/ready")]
    System.Threading.Tasks.Task<bool> ready();

    [GetRequest("/ratio")]
    System.Threading.Tasks.Task<double> ratio();

    [GetRequest("/numbers")]
    System.Threading.Tasks.Task<System.Collections.Generic.List<int>> numbers();

    [GetRequest("/shelf")]
    System.Threading.Tasks.Task<Models.Shelf> shelf();

    [GetRequest("/first")]
    System.Threading.Tasks.ValueTask<Models.ShelfItem> first();

    [GetRequest("/wrong-items")]
    System.Threading.Tasks.Task<List<Models.ShelfItem>> wrongItems();

    [GetRequest("/wrong-count")]
    System.Threading.Tasks.Task<int> wrongCount();

    [GetRequest("/wrong-shelf")]
    System.Threading.Tasks.Task<Models.Shelf> wrongShelf();
}
