import java.util.Map;
import models.UserSaved;

@Service
class UserFormService {
    @Needs
    private final UsersClient usersClient;
    ApplicationEventPublisher applicationEventPublisher;
    String name = "";

    void save() {
        Map<?, ?> answer = (Map<?, ?>) usersClient.save(name);
        applicationEventPublisher.publishEvent(new UserSaved((String) answer.get("name")), false);
    }
}
