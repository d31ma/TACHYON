final class TacPause {
    private TacPause() {
    }

    static void sleep(long milliseconds) throws InterruptedException {
        TacHost.pause(milliseconds);
    }
}
