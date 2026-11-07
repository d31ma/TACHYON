class ApplicationEventPublisher {
    private final TacActivation owner;

    ApplicationEventPublisher() {
        this(null);
    }

    ApplicationEventPublisher(TacActivation owner) {
        this.owner = owner;
    }

    public <T> T publishEvent(T value) {
        return publishEvent(value, false);
    }

    public <T> T publishEvent(T value, boolean retain) {
        TacEvents.requireOwner(owner);
        TacEvents.publish(value, retain);
        return value;
    }
}
