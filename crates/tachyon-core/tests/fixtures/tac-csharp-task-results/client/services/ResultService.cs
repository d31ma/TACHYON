[Service]
public sealed class ResultService
{
    [Needs]
    private readonly ResultClient _resultClient;

    public string Summary = "";

    public System.Threading.Tasks.Task<List<Models.ShelfItem>> items()
    {
        return _resultClient.items();
    }

    public System.Threading.Tasks.ValueTask<Models.ShelfItem[]> array()
    {
        return new System.Threading.Tasks.ValueTask<Models.ShelfItem[]>(_resultClient.array());
    }

    public System.Threading.Tasks.Task<int> count()
    {
        return _resultClient.count();
    }

    public System.Threading.Tasks.Task<string> label()
    {
        return _resultClient.label();
    }

    public System.Threading.Tasks.Task<bool> ready()
    {
        return _resultClient.ready();
    }

    public System.Threading.Tasks.Task<double> ratio()
    {
        return _resultClient.ratio();
    }

    public System.Threading.Tasks.Task<System.Collections.Generic.List<int>> numbers()
    {
        return _resultClient.numbers();
    }

    public System.Threading.Tasks.Task<Models.Shelf> shelf()
    {
        return _accept(_resultClient.shelf());
    }

    public System.Threading.Tasks.ValueTask<Models.ShelfItem> first()
    {
        return _resultClient.first();
    }

    public System.Threading.Tasks.Task<List<Models.ShelfItem>> wrongItems()
    {
        return _resultClient.wrongItems();
    }

    public System.Threading.Tasks.Task<int> wrongCount()
    {
        return _resultClient.wrongCount();
    }

    public System.Threading.Tasks.Task<Models.Shelf> wrongShelf()
    {
        return _accept(_resultClient.wrongShelf());
    }

    private async System.Threading.Tasks.Task<Models.Shelf> _accept(System.Threading.Tasks.Task<Models.Shelf> pending)
    {
        var shelf = await pending;
        Summary = shelf.Label + ": " + shelf.Detail.Note;
        return shelf;
    }
}
