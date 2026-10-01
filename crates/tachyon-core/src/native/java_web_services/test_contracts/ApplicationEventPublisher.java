class ApplicationEventPublisher {
    public <T> T publishEvent(T value) {
        return publishEvent(value, false);
    }

    public <T> T publishEvent(T value, boolean retain) {
        throw new IllegalStateException("Java Tests must supply an ApplicationEventPublisher fake.");
    }
}
