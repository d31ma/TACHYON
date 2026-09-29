[Service]
public class CatalogService {
    readonly CatalogClient _catalog = new CatalogClient();
    public int count = 0;

    public System.Threading.Tasks.Task<BehaviorJSON> All() { return _catalog.All(); }
}
