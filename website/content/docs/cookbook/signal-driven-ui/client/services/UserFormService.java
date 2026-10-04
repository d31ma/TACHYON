import java.util.Map;
import models.UserSaved;

@Service
class UserFormService {
    private final UsersClient usersClient = new UsersClient();
    ApplicationEventPublisher applicationEventPublisher;
    String name = "";

    void save() {
        Map<?, ?> answer = (Map<?, ?>) usersClient.save(name);
        applicationEventPublisher.publishEvent(new UserSaved((String) answer.get("name")), false);
    }
}
