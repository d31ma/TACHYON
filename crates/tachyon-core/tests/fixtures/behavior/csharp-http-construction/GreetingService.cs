[Service]
public sealed class GreetingService {
    [Needs]
    private readonly GreetingClient greetingClient;

    public System.Threading.Tasks.Task<BehaviorJSON> read() {
        return greetingClient.read();
    }
}

