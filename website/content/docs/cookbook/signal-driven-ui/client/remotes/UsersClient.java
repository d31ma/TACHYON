import java.util.List;

@HTTPClient
interface UsersClient {
    @GetRequest("/users")
    List<String> names();

    @PostRequest("/users")
    Object save(@RequestBody Object name);
}
