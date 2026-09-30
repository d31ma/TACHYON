[Service]
public class CatalogService {
    readonly CatalogClient _catalog = new CatalogClient();
    public int count = 0;

    public async System.Threading.Tasks.Task Load() {
        var answer = await _catalog.All();
        count = answer.Get("items").Items.Length;
    }
}
