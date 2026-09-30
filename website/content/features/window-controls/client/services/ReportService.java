@Service
class ReportService {
    private final WindowControls windowControls = new WindowControls();
    String problem = "";

    void minimize() {
        keepProblem(windowControls::minimize);
    }

    void resize() {
        keepProblem(() -> windowControls.resize(1280, 800));
    }

    void rename() {
        keepProblem(() -> windowControls.setTitle("Quarterly report"));
    }

    private void keepProblem(Runnable call) {
        try {
            call.run();
        } catch (IllegalStateException refusal) {
            problem = refusal.getMessage();
        }
    }
}
