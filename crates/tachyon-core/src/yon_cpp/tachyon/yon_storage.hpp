namespace tachyon {

class BehaviorMemory {
public:
    static BehaviorCacheAdapter cache() {
        auto entries = std::make_shared<std::map<std::string, BehaviorCacheEntry>>();
        BehaviorCacheAdapter adapter;
        adapter.get = [entries](const std::string& key, const BehaviorExecution&) -> std::optional<BehaviorCacheEntry> {
            auto found = entries->find(key);
            if (found == entries->end()) {
                return std::nullopt;
            }
            return found->second;
        };
        adapter.set = [entries](const std::string& key, const BehaviorCacheEntry& entry, std::size_t maxEntries, const BehaviorExecution&) {
            bool full = entries->count(key) == 0 && !entries->empty() && entries->size() >= maxEntries;
            if (full) {
                entries->erase(entries->begin());
            }
            (*entries)[key] = entry;
        };
        adapter.remove = [entries](const std::string& key, const BehaviorExecution&) {
            entries->erase(key);
        };
        return adapter;
    }

    static BehaviorEntityAdapter entities() {
        return [](const json::JsonNode& command, const BehaviorExecution&) {
            json::JsonNode answer = TacMemoryEntities::execute(command);
            bool begun = command["operation"].getValue<std::string>() == "transaction" && command["action"].getValue<std::string>() == "begin";
            return begun ? answer["transaction"] : answer;
        };
    }
};

}

namespace tachyon::yon {

class YonAdapters {
public:
    static json::JsonNode cache(const BehaviorContext& context, const json::JsonNode& request) {
        const BehaviorAdapters& adapters = context.adapters();
        std::string name = request["cache"].getValue<std::string>();
        auto found = adapters.caches.find(name);
        if (found == adapters.caches.end()) {
            throw TacCodedError("The Setup of this behavior context has no cache adapter named '" + name + "'.", "invalid");
        }
        const BehaviorCacheAdapter& adapter = found->second;
        if (!adapter.get || !adapter.set || !adapter.remove) {
            throw TacCodedError("The cache adapter '" + name + "' sets get, set and remove.", "invalid");
        }
        BehaviorExecution execution = context.execution();
        authorize(adapters, request["call"], execution);
        std::string operation = request["op"].getValue<std::string>();
        std::string key = request["key"].getValue<std::string>();
        if (operation == "cache.get") {
            return read(adapter, key, execution);
        }
        if (operation == "cache.set") {
            BehaviorCacheEntry entry{request["value"], request["expiresAt"].getValue<double>()};
            adapter.set(key, entry, static_cast<std::size_t>(request["maxEntries"].getValue<double>()), execution);
            return json::JsonNode(nullptr);
        }
        adapter.remove(key, execution);
        return json::JsonNode(nullptr);
    }

    static json::JsonNode entity(const BehaviorContext& context, const json::JsonNode& request) {
        const BehaviorAdapters& adapters = context.adapters();
        std::string name = request["adapter"].getValue<std::string>();
        auto found = adapters.entities.find(name);
        if (found == adapters.entities.end() || !found->second) {
            throw TacCodedError("The Setup of this behavior context has no entity adapter named '" + name + "'.", "invalid");
        }
        const json::JsonNode& command = request["command"];
        BehaviorExecution execution = context.execution();
        std::string operation = command["operation"].getValue<std::string>();
        if (operation == "query") {
            authorizeEntity(adapters, command, execution);
        }
        json::JsonNode answer = found->second(command, execution);
        bool begun = operation == "transaction" && command["action"].getValue<std::string>() == "begin";
        if (!begun) {
            return answer;
        }
        json::JsonObject opened;
        opened["transaction"] = answer;
        return opened;
    }

private:
    static json::JsonNode read(const BehaviorCacheAdapter& adapter, const std::string& key, const BehaviorExecution& execution) {
        std::optional<BehaviorCacheEntry> entry = adapter.get(key, execution);
        json::JsonObject answer;
        answer["hit"] = entry.has_value();
        if (entry.has_value()) {
            answer["value"] = entry->value;
            answer["expiresAt"] = entry->expiresAt;
        }
        return answer;
    }

    static void authorize(const BehaviorAdapters& adapters, const json::JsonNode& written, const BehaviorExecution& execution) {
        if (!adapters.authorize) {
            throw TacCodedError("A Yon C++ Setup that gives caches also sets adapters.authorize, which answers true for each cache call it allows.", "invalid");
        }
        BehaviorCacheCall call{
            written["operation"].getValue<std::string>(),
            written["cache"].getValue<std::string>(),
            written["owner"].getValue<std::string>(),
            written["method"].getValue<std::string>(),
            written["keyArguments"],
        };
        if (!adapters.authorize(call, execution)) {
            throw TacCodedError("Behavior cache access was not authorized.", "unauthorized");
        }
    }

    static void authorizeEntity(const BehaviorAdapters& adapters, const json::JsonNode& command, const BehaviorExecution& execution) {
        if (!adapters.authorizeEntity) {
            throw TacCodedError("A Yon C++ Setup whose Store runs queries also sets adapters.authorizeEntity, which answers true for each query it allows.", "invalid");
        }
        if (!adapters.authorizeEntity(command, execution)) {
            throw TacCodedError("Store query was not authorized.", "unauthorized");
        }
    }
};

class YonStorage : public TacHost::Storage {
public:
    explicit YonStorage(const BehaviorContext& context) : context(context), previous(TacHost::current()) {
        TacHost::current() = this;
    }

    ~YonStorage() override {
        TacHost::current() = previous;
    }

    YonStorage(const YonStorage&) = delete;
    YonStorage& operator=(const YonStorage&) = delete;

    json::JsonNode answer(const json::JsonNode& request) override {
        if (request["op"].getValue<std::string>() == "entity") {
            return YonAdapters::entity(context, request);
        }
        return YonAdapters::cache(context, request);
    }

    json::JsonNode partition() override {
        return json::JsonNode(context.partition());
    }

private:
    const BehaviorContext& context;
    TacHost::Storage* previous;
};

class YonStores : public TacHost::Storage {
public:
    static void keep(const std::string& entity, const BehaviorContext& context) {
        TacHost::stores() = &router();
        opened()[entity] = context;
    }

    static void release(const std::string& entity) {
        opened().erase(entity);
    }

    json::JsonNode answer(const json::JsonNode& request) override {
        std::string entity = request["command"]["entity"].getValue<std::string>();
        auto found = opened().find(entity);
        if (found == opened().end()) {
            throw TacCodedError("The Store that keeps " + entity + " is not open.", "cancelled");
        }
        return YonAdapters::entity(found->second, request);
    }

    json::JsonNode partition() override {
        return json::JsonNode(nullptr);
    }

private:
    static YonStores& router() {
        static YonStores kept;
        return kept;
    }

    static std::map<std::string, BehaviorContext>& opened() {
        static std::map<std::string, BehaviorContext> contexts;
        return contexts;
    }
};

template <typename Setup>
class YonStore {
public:
    YonStore(std::string entity, std::string partition) : entity(std::move(entity)) {
        BehaviorOptions options;
        options.partition = std::move(partition);
        context = Behaviors::create<Setup>(options);
        YonStores::keep(this->entity, context);
    }

    ~YonStore() {
        YonStores::release(entity);
        context.dispose();
    }

    YonStore(const YonStore&) = delete;
    YonStore& operator=(const YonStore&) = delete;

private:
    std::string entity;
    BehaviorContext context;
};

}

using tachyon::BehaviorMemory;
