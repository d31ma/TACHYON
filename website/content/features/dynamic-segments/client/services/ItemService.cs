[Service]
public sealed class ItemService {
    public string IdFrom(string path) => path.Substring(path.LastIndexOf('/') + 1);
}
