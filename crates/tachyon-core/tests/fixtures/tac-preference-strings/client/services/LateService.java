import models.PlatformChanged;

@Service
@Lifetime({"value":"prototype"})
class LateService {
    String current = "waiting";

    @OnEvent({"replay":true})
    void changed(PlatformChanged value) {
        current = value.platform();
    }
}
