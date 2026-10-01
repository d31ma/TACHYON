import models.PlatformChanged;

@Service
class LateService {
    String current = "waiting";

    @OnEvent({"replay":true})
    void changed(PlatformChanged value) {
        current = value.platform();
    }
}
