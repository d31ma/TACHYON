final class TacHttpTickets {
    private static final String OUTSIDE_TASK = "{\"error\":{\"code\":\"HTTP_REQUEST\"}}";
    private static final java.util.Map<Integer, TacTask> WAITING = new java.util.HashMap<>();
    private static int last;

    private TacHttpTickets() {
    }

    static String exchange(String request) {
        TacTask task = TacTasks.current();
        if (task == null || task.owner == null) {
            return OUTSIDE_TASK;
        }
        last++;
        int ticket = last;
        TacActivation owner = task.owner;
        TacBridge.release();
        String answer = await(ticket, task, request);
        TacBridge.reenter(owner);
        return answer;
    }

    static void settle(int ticket, String answer) {
        TacTask task = WAITING.remove(ticket);
        if (task == null) {
            return;
        }
        org.teavm.interop.AsyncCallback<String> resume = task.resume;
        task.resume = null;
        task.ticket = 0;
        TacTask outer = TacTasks.current();
        TacTasks.enter(task);
        try {
            resume.complete(answer);
        } finally {
            TacTasks.enter(outer);
        }
    }

    static void cancel(TacTask task) {
        if (WAITING.remove(task.ticket) == null) {
            return;
        }
        task.resume = null;
        TacHost.httpCancel(task.ticket);
        task.ticket = 0;
    }

    @org.teavm.interop.Async
    private static native String await(int ticket, TacTask task, String request);

    private static void await(int ticket, TacTask task, String request, org.teavm.interop.AsyncCallback<String> resume) {
        task.ticket = ticket;
        task.resume = resume;
        WAITING.put(ticket, task);
        TacHost.httpStart(ticket, request);
    }
}
