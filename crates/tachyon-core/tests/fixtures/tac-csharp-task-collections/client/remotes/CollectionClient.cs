[HTTPClient({"path":"/api/collections","adapter":"collections","setup":"CollectionSetup"})]
public interface CollectionClient
{
    [GetRequest]
    System.Threading.Tasks.Task<Models.MessagePage> read();

    [GetRequest("/empty")]
    System.Threading.Tasks.Task<Models.MessagePage> empty();

    [GetRequest("/nullable")]
    System.Threading.Tasks.Task<Models.MessagePage> nullable();

    [GetRequest("/missing")]
    System.Threading.Tasks.Task<Models.MessagePage> missing();

    [GetRequest("/wrong-array")]
    System.Threading.Tasks.Task<Models.MessagePage> wrongArray();

    [GetRequest("/missing-member")]
    System.Threading.Tasks.Task<Models.MessagePage> missingMember();

    [GetRequest("/wrong-member")]
    System.Threading.Tasks.Task<Models.MessagePage> wrongMember();

    [GetRequest("/extra-member")]
    System.Threading.Tasks.Task<Models.MessagePage> extraMember();

    [GetRequest("/null-member")]
    System.Threading.Tasks.Task<Models.MessagePage> nullMember();

    [GetRequest("/slow")]
    System.Threading.Tasks.Task<Models.MessagePage> slow();
}
