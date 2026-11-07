import java.util.ArrayList;
import java.util.List;
import models.UserSaved;

@Service
class UserListService {
    @Needs
    private final UsersClient usersClient;
    List<String> names = new ArrayList<>();

    @OnStart
    void load() { names.addAll(usersClient.names()); }

    @OnEvent
    void userSaved(UserSaved saved) { names.add(saved.name()); }
}
