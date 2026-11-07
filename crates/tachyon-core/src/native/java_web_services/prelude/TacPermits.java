final class TacPermits {
    private int available;

    TacPermits(int permits, boolean fair) {
        available = permits;
    }

    boolean tryAcquire() {
        if (available <= 0) {
            return false;
        }
        available--;
        return true;
    }

    boolean tryAcquire(long timeout, java.util.concurrent.TimeUnit unit) throws InterruptedException {
        return tryAcquire();
    }

    void release() {
        available++;
    }
}
