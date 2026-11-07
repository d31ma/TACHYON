public class TacServices {
    public static void main(String[] arguments) {
    }

    @org.teavm.jso.JSExport
    public static String tacInvoke(String request) {
        return TacTasks.run(request);
    }

    @org.teavm.jso.JSExport
    public static void tacHttpSettle(int ticket, String answer) {
        TacHttpTickets.settle(ticket, answer);
    }
}
