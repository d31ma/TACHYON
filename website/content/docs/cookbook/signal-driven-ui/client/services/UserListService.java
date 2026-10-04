import java.util.ArrayList;
import java.util.List;
import models.UserSaved;

@Service
class UserListService {
    List<String> names = new ArrayList<>();

    @OnEvent
    void userSaved(UserSaved saved) { names.add(saved.name()); }
}
