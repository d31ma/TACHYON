[Service]
public sealed class CollectionService
{
    [Needs]
    private readonly CollectionClient _collectionClient;

    public string First = "";

    public System.Threading.Tasks.Task<Models.MessagePage> ask()
    {
        return _accept(_collectionClient.read());
    }

    public System.Threading.Tasks.Task<Models.MessagePage> empty()
    {
        return _collectionClient.empty();
    }

    public System.Threading.Tasks.Task<Models.MessagePage> nullable()
    {
        return _collectionClient.nullable();
    }

    public System.Threading.Tasks.Task<Models.MessagePage> nullList()
    {
        return _collectionClient.nullList();
    }

    public System.Threading.Tasks.Task<Models.MessagePage> nullableLeaf()
    {
        return _collectionClient.nullableLeaf();
    }

    public System.Threading.Tasks.Task<Models.MessagePage> wrongNullableLeaf()
    {
        return _accept(_collectionClient.wrongNullableLeaf());
    }

    public System.Threading.Tasks.Task<Models.MessagePage> missing()
    {
        return _accept(_collectionClient.missing());
    }

    public System.Threading.Tasks.Task<Models.MessagePage> wrongArray()
    {
        return _accept(_collectionClient.wrongArray());
    }

    public System.Threading.Tasks.Task<Models.MessagePage> missingMember()
    {
        return _accept(_collectionClient.missingMember());
    }

    public System.Threading.Tasks.Task<Models.MessagePage> wrongMember()
    {
        return _accept(_collectionClient.wrongMember());
    }

    public System.Threading.Tasks.Task<Models.MessagePage> extraMember()
    {
        return _accept(_collectionClient.extraMember());
    }

    public System.Threading.Tasks.Task<Models.MessagePage> nullMember()
    {
        return _accept(_collectionClient.nullMember());
    }

    public System.Threading.Tasks.Task<Models.MessagePage> slow()
    {
        return _accept(_collectionClient.slow());
    }

    private async System.Threading.Tasks.Task<Models.MessagePage> _accept(System.Threading.Tasks.Task<Models.MessagePage> pending)
    {
        var answer = await pending;
        First = answer.Items[0].Message;
        return answer;
    }
}
