namespace tachyon {

class TacEvent {
public:
    std::string name;
    std::string value;
    bool retain = false;
};

class TacOwner {
public:
    std::string route;
    std::string service;
    std::string id;
    std::shared_ptr<void> instance;
    const TacServiceType* type = nullptr;
    std::deque<TacEvent> queue;
    bool disposed = false;

    bool matches(const json::JsonNode& request) const {
        return route == request["route"].getValue<std::string>() && service == request["service"].getValue<std::string>();
    }

    void runHooks(const char* list) {
        const json::JsonNode& names = type->metadata[list];
        if (names.getValueKind() != json::JsonValueKind::Array) {
            return;
        }
        for (const json::JsonNode& name : names.items()) {
            const TacMethod* hook = type->hook(name.getValue<std::string>());
            if (hook != nullptr) {
                hook->call(instance.get(), json::JsonArray());
            }
        }
    }
};

}
