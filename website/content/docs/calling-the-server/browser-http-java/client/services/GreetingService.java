@Service
class GreetingService {
    @Needs
    private final GreetingClient greetingClient;
    @Needs
    private final OverrideClient overrideClient;
    String message = "";
    String custom = "";
    int loads = 0;
    int clears = 0;

    void ask() {
        loads++;
        message = greetingClient.read().message();
    }

    void clear() {
        greetingClient.clear();
        clears++;
    }

    void askCustom() {
        custom = overrideClient.read().message();
    }
}
