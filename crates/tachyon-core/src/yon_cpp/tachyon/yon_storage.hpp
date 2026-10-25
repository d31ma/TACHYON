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
            return TacAdapters::entity(context, request);
        }
        return TacAdapters::cache(context, request);
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
        return TacAdapters::entity(found->second, request);
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
        static auto* contexts = new std::map<std::string, BehaviorContext>();
        return *contexts;
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
