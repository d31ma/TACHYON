[HTTPClient({"adapter":"users","setup":"UsersSetup"})]
public interface UsersClient {
    [PostRequest("/users")]
    System.Threading.Tasks.Task<Models.UserSaved> Save([RequestBody] string name);

    [GetRequest("/users")]
    System.Threading.Tasks.Task<BehaviorJSON> All();
}
