final class TacEvents {
    private static final int MOST_QUEUED = 128;
    private static final int MOST_REPUBLISHED = 32;
    private static final int LARGEST_EVENT = 65536;
    private static final String FAILED = "Tachyon web event delivery failed.";
    private static TacActivation executing;

    private TacEvents() {
    }

    static void runFor(TacActivation activation) {
        executing = activation;
    }

    static void requireOwner(TacActivation owner) {
        checkOwner(owner);
    }

    private static void checkOwner(TacActivation owner) {
        if (owner != null && (owner.disposed || executing != owner)) {
            throw new IllegalStateException("Java event publisher owner is disposed or inactive.");
        }
    }

    static void publish(Object value, boolean retain) {
        TacActivation owner = executing;
        String name = TacModels.name(value);
        Object plain = TacModels.plain(value);
        boolean accepted = owner != null && !owner.disposed && owner.queue.size() < MOST_QUEUED
                && !name.isEmpty() && plain instanceof java.util.Map<?, ?>;
        if (!accepted) {
            failed();
            return;
        }
        String json = TacJsonWriter.write(plain);
        if (json.getBytes(java.nio.charset.StandardCharsets.UTF_8).length > LARGEST_EVENT) {
            failed();
            return;
        }
        owner.queue.add(new TacEvent(name, json, retain));
    }

    static void republish(Object result) {
        java.util.List<Object> items = new java.util.ArrayList<>();
        if (result instanceof Iterable<?> many) {
            many.forEach(items::add);
        } else {
            items.add(result);
        }
        for (Object item : items.subList(0, Math.min(items.size(), MOST_REPUBLISHED))) {
            if (item != null) {
                publish(item, false);
            }
        }
    }

    static void drain() {
        TacActivation owner = executing;
        executing = null;
        int sent = 0;
        while (owner != null && !owner.disposed && !owner.queue.isEmpty()) {
            if (sent++ == MOST_QUEUED) {
                owner.queue.clear();
                failed();
                return;
            }
            TacEvent event = owner.queue.poll();
            TacHost.emit("{\"route\":" + TacJsonWriter.quote(owner.route) + ",\"service\":" + TacJsonWriter.quote(owner.service)
                    + ",\"owner\":" + TacJsonWriter.quote(owner.owner) + ",\"name\":" + TacJsonWriter.quote(event.name)
                    + ",\"value\":" + event.json + ",\"retain\":" + event.retain + "}");
        }
    }

    private static void failed() {
        System.out.println(FAILED);
    }
}
