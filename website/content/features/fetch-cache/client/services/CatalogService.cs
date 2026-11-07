[Service]
public sealed class CatalogService {
    [Needs]
    private readonly ProductsClient _products;
    public int Count = 0;

    public async System.Threading.Tasks.Task Load() {
        var answer = await _products.List();
        Count = answer.Get("items").Items.Length;
    }

    public async System.Threading.Tasks.Task Add() {
        await _products.Save(BehaviorJSON.Parse("{\"name\":\"anvil\"}"));
    }
}
