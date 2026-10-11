@Service
class CounterService {
    @Needs
    private final GreetingClient greetingClient;
    int count = 0;
    String message = "";

    void increment() {
        count += 1;
    }

    void askServer() {
        message = greetingClient.getGreeting().message();
    }
}
