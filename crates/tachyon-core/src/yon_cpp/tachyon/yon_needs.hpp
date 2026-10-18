namespace tachyon::yon {

class YonNeeds {
public:
    template <typename Provider>
    static Provider& of();
};

class YonMade {
public:
    std::shared_ptr<void> owner;
    std::function<void(YonEvents&)> listen;
    std::function<void()> start;
    std::function<void()> stop;
};

class YonContainer {
public:
    static constexpr std::size_t MAX_INSTANCES = 4096;

    template <typename Provider>
    static Provider& shared(Provider*& slot, void (*describe)(Provider&, YonMade&)) {
        if (slot == nullptr) {
            slot = &fresh<Provider>(describe);
        }
        return *slot;
    }

    template <typename Provider>
    static Provider& fresh(void (*describe)(Provider&, YonMade&)) {
        if (made().size() >= MAX_INSTANCES) {
            throw std::runtime_error("Yon dependency resolution made more than 4096 objects");
        }
        std::shared_ptr<Provider> owner = std::make_shared<Provider>();
        YonMade entry;
        entry.owner = owner;
        describe(*owner, entry);
        made().push_back(std::move(entry));
        return *owner;
    }

    static std::vector<YonMade>& made() {
        static std::vector<YonMade> objects;
        return objects;
    }
};

class YonListener {
public:
    template <typename Owner, typename Result, typename Event>
    static json::JsonNode call(Owner& owner, Result (Owner::*method)(Event), const json::JsonNode& value) {
        std::decay_t<Event> event = json::JsonSerializer::deserialize<std::decay_t<Event>>(value);
        if constexpr (std::is_void_v<Result>) {
            (owner.*method)(event);
            return json::JsonNode(nullptr);
        } else {
            return json::JsonSerializer::serializeToNode((owner.*method)(event));
        }
    }

    template <typename Owner, typename Result, typename Event>
    static json::JsonNode call(Owner& owner, Result (Owner::*method)(Event) const, const json::JsonNode& value) {
        std::decay_t<Event> event = json::JsonSerializer::deserialize<std::decay_t<Event>>(value);
        if constexpr (std::is_void_v<Result>) {
            (owner.*method)(event);
            return json::JsonNode(nullptr);
        } else {
            return json::JsonSerializer::serializeToNode((owner.*method)(event));
        }
    }
};

class YonInvocation {
public:
    YonInvocation() = default;
    YonInvocation(const YonInvocation&) = delete;
    YonInvocation& operator=(const YonInvocation&) = delete;

    void start() {
        std::vector<YonMade>& objects = YonContainer::made();
        for (YonMade& object : objects) {
            listen(object);
        }
        for (std::size_t index = 0; index < objects.size(); index += 1) {
            begin(objects[index]);
            started = index + 1;
        }
        YonEvents::current().drain();
        YonDelivery::deliver();
        YonEvents::current().drain();
    }

    void drain() {
        YonEvents::current().drain();
    }

    ~YonInvocation() {
        YonEvents::current().close();
        std::vector<YonMade>& objects = YonContainer::made();
        for (std::size_t index = started; index > 0; index -= 1) {
            end(objects[index - 1]);
        }
        while (!objects.empty()) {
            objects.pop_back();
        }
    }

private:
    std::size_t started = 0;

    static void listen(YonMade& object) {
        if (object.listen) {
            object.listen(YonEvents::current());
        }
    }

    static void begin(YonMade& object) {
        if (object.start) {
            object.start();
        }
    }

    static void end(YonMade& object) {
        if (!object.stop) {
            return;
        }
        try {
            object.stop();
        } catch (...) {
            YonEvents::report("Yon local lifecycle cleanup failed.");
        }
    }
};

}
