using Models;

[Service]
public class UserListService {
    public List<string> Names = new List<string>();

    [OnEvent]
    public void UserSaved(UserSaved saved) { Names.Add(saved.Name); }
}
