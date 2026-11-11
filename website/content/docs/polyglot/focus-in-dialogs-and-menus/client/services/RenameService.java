@Service
class RenameService {
    private final Focus focus = new Focus();
    boolean editing = false;
    String problem = "";

    void open() {
        editing = true;
        keepProblem(() -> focus.trap("dialog"));
        keepProblem(() -> focus.move("name"));
    }

    void close() {
        editing = false;
        keepProblem(focus::release);
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
