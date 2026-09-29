final class Clock {
    private static final String API = "clock";

    double nowMs() {
        return TacWebHost.number(API, "now");
    }

    double monotonicMs() {
        return TacWebHost.number(API, "monotonic");
    }

    String timeZone() {
        return TacWebHost.text(API, "timeZone");
    }
}
