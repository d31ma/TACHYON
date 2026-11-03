final class TacPreferenceScope {
    private boolean disposed;

    void check() {
        requireActive();
    }

    private void requireActive() {
        if (disposed) {
            throw new IllegalStateException("PreferenceStrings owner is disposed.");
        }
    }

    void dispose() {
        disposed = true;
    }
}
