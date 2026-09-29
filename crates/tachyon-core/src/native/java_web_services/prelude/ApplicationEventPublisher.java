final class ApplicationEventPublisher {
    <T> T publishEvent(T value) {
        return publishEvent(value, false);
    }

    <T> T publishEvent(T value, boolean retain) {
        TacEvents.publish(value, retain);
        return value;
    }
}
