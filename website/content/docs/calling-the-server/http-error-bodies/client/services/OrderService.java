import models.Problem;

@Service
class OrderService {
    @Needs
    private final OrderClient orderClient;
    int status = 0;
    String title = "";
    int mistakes = 0;

    void send() {
        Behaviors.Failure failure = failure();
        show(Behaviors.errorStatus(failure), problemOf(failure));
    }

    private Behaviors.Failure failure() {
        try {
            orderClient.send(java.util.Map.of("name", "", "level", 12));
            return new Behaviors.Failure("none", null, "no failure", 0, null);
        } catch (Behaviors.Failure failure) {
            return failure;
        }
    }

    private Problem problemOf(Behaviors.Failure failure) {
        return Behaviors.errorBody(failure, TacModels::readProblem).orElse(new Problem("", 0, java.util.List.of()));
    }

    private void show(int status, Problem problem) {
        this.status = status;
        title = problem.title();
        mistakes = problem.errors().size();
    }
}
