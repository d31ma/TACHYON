[HTTPClient({"path":"/api/notes","setup":"NotesSetup"})]
public interface NotesClient
{
    [GetRequest("/first")]
    System.Threading.Tasks.Task<BehaviorJSON> first();

    [GetRequest("/missing")]
    System.Threading.Tasks.Task<BehaviorJSON> missing();

    [GetRequest("/slow")]
    System.Threading.Tasks.Task<BehaviorJSON> slow();

    [GetRequest("/large")]
    System.Threading.Tasks.Task<BehaviorJSON> large();
}
