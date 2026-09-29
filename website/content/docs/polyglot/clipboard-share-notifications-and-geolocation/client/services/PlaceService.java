@Service
class PlaceService {
    private final Geolocation geolocation = new Geolocation();
    private final Clipboard clipboard = new Clipboard();
    String place = "";
    String problem = "";

    void locate() {
        keepProblem(geolocation::currentPosition);
    }

    void copy() {
        keepProblem(() -> clipboard.writeText(place));
    }

    @OnEvent
    void found(PositionFound found) {
        place = found.latitude() + ", " + found.longitude();
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
