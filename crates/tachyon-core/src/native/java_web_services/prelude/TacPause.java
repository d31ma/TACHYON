final class TacPause {
    private static final long NANOSECONDS_PER_MILLISECOND = 1000000L;

    private TacPause() {
    }

    static void sleep(long milliseconds) throws InterruptedException {
        long until = System.nanoTime() + milliseconds * NANOSECONDS_PER_MILLISECOND;
        long now = System.nanoTime();
        while (now - until < 0) {
            now = System.nanoTime();
        }
    }
}
