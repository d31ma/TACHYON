final class TacTask {
    final int id;
    TacActivation owner;
    String answer;
    int ticket;
    org.teavm.interop.AsyncCallback<String> resume;

    TacTask(int id) {
        this.id = id;
    }
}
