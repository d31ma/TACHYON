[HTTPClient]
public interface UsersClient {
    [PostRequest("/users")]
    System.Threading.Tasks.Task<Models.UserSaved> Save([RequestBody] string name);
}
