using Models;

[Service]
public sealed class UserFormService {
    [Needs]
    private readonly UsersClient _usersClient;
    private readonly ApplicationEventPublisher applicationEventPublisher;
    public string Name = "";

    public async System.Threading.Tasks.Task Save() {
        UserSaved saved = await _usersClient.Save(Name);
        applicationEventPublisher.PublishEvent(saved, false);
    }
}
