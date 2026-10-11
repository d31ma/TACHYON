[Service]
public class CatalogService {
    [Needs]
    private readonly CatalogClient _catalog;
    public int count = 0;

    public async System.Threading.Tasks.Task Load() {
        var answer = await _catalog.All();
        count = answer.Get("items").Items.Length;
    }
}
