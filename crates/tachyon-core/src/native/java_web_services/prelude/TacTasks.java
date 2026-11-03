final class TacTasks {
    private static final String CANCELLED = "{\"error\":\"The service was removed before its task finished.\",\"code\":\"CANCELLED\"}";
    private static final java.util.Map<Integer, TacTask> PARKED = new java.util.LinkedHashMap<>();
    private static int last;
    private static TacTask current;

    private TacTasks() {
    }

    static String run(String request) {
        last++;
        TacTask task = new TacTask(last);
        TacTask outer = current;
        current = task;
        try {
            org.teavm.runtime.Fiber.start(() -> finish(task, TacBridge.handle(request)), true);
        } finally {
            current = outer;
        }
        if (task.answer != null) {
            return task.answer;
        }
        PARKED.put(task.id, task);
        return "{\"task\":" + task.id + "}";
    }

    static TacTask current() {
        return current;
    }

    static void enter(TacTask task) {
        current = task;
    }

    static void allowWaiting(TacActivation owner) {
        if (current != null) {
            current.owner = owner;
        }
    }

    static void cancel(String owner) {
        for (TacTask task : new java.util.ArrayList<>(PARKED.values())) {
            if (task.owner != null && task.owner.owner.equals(owner)) {
                PARKED.remove(task.id);
                TacHttpTickets.cancel(task);
                TacHost.taskDone(task.id, CANCELLED);
            }
        }
    }

    private static void finish(TacTask task, String answer) {
        task.answer = answer;
        if (PARKED.remove(task.id) != null) {
            TacHost.taskDone(task.id, answer);
        }
    }
}
