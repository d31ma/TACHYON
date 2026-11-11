@Service
class AlertsService {
    private static final String SERVER_KEY = "BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8U";
    private final Push push = new Push();
    String endpoint = "";
    String problem = "";

    void turnOn() {
        keepProblem(() -> push.subscribe(SERVER_KEY));
    }

    void turnOff() {
        keepProblem(push::unsubscribe);
    }

    @OnEvent
    void subscribed(PushSubscribed subscribed) {
        endpoint = subscribed.endpoint();
    }

    @OnEvent
    void unsubscribed(PushUnsubscribed unsubscribed) {
        endpoint = "";
    }

    @OnEvent
    void failed(BrowserCallFailed failed) {
        problem = failed.message();
    }

    private void keepProblem(Runnable call) {
        try {
            call.run();
        } catch (IllegalStateException refusal) {
            problem = refusal.getMessage();
        }
    }
}
