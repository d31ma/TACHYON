final class TacHook {
    final boolean destroy;
    final Runnable invoke;

    TacHook(boolean destroy, Runnable invoke) {
        this.destroy = destroy;
        this.invoke = invoke;
    }
}
