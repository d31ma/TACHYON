using Models;

[Service]
public class UserListService {
    [Needs]
    private readonly UsersClient _usersClient;
    public List<string> Names = new List<string>();

    [OnEvent]
    public async System.Threading.Tasks.Task UserSaved(UserSaved saved) {
        BehaviorJSON users = await _usersClient.All();
        Names = users.Items.Select(user => user.Get("name").Text).ToList();
    }
}
