[Service]
public sealed class CatalogService {
    private readonly ProductsClient products = new ProductsClient();
    public int Count = 0;

    public async System.Threading.Tasks.Task Load() {
        var answer = await products.List();
        Count = answer.Get("items").Items.Length;
    }

    public async System.Threading.Tasks.Task Add() {
        await products.Save(BehaviorJSON.Parse("{\"name\":\"anvil\"}"));
    }
}
