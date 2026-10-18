namespace tachyon {

class TacAdapters {
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

    static void allowed(const BehaviorContext& context, const json::JsonNode& request) {
        const BehaviorAdapters& adapters = context.adapters();
        bool cached = request["op"].getValue<std::string>().rfind("cache.", 0) == 0;
        if (cached && adapters.authorize) {
            authorize(adapters, request["call"], context.execution());
        }
        bool queried = !cached && request["command"]["operation"].getValue<std::string>() == "query";
        if (queried && adapters.authorizeEntity) {
            authorizeEntity(adapters, request["command"], context.execution());
        }
    }

    static const BehaviorHTTPAdapter* http(const BehaviorContext* context, const std::string& name) {
        if (context == nullptr) {
            return nullptr;
        }
        const std::map<std::string, BehaviorHTTPAdapter>& http = context->adapters().http;
        auto found = http.find(name);
        return found == http.end() ? nullptr : &found->second;
    }

    static http::HttpResponseMessage answered(const json::JsonNode& value) {
        http::HttpResponseMessage response;
        response.content = http::HttpContent(value.toJsonString());
        return response;
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
            throw TacCodedError("A C++ Setup that gives caches also sets adapters.authorize, which answers true for each cache call it allows.", "invalid");
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
            throw TacCodedError("A C++ Setup whose Store runs queries on its own entity adapter also sets adapters.authorizeEntity, which answers true for each query it allows.", "invalid");
        }
        if (!adapters.authorizeEntity(command, execution)) {
            throw TacCodedError("Store query was not authorized.", "unauthorized");
        }
    }
};

class TacLazyContext {
public:
    TacLazyContext() = default;

    explicit TacLazyContext(std::function<BehaviorContext()> opener) : state(std::make_shared<TacOpening>(std::move(opener))) {}

    template <typename Setup>
    static TacLazyContext of(std::string partition) {
        return TacLazyContext([partition]() {
            BehaviorOptions options;
            options.partition = partition;
            return Behaviors::create<Setup>(options);
        });
    }

    const BehaviorContext* get() const {
        if (state == nullptr) {
            return nullptr;
        }
        if (!state->context.isOpen()) {
            state->context = state->opener();
        }
        return &state->context;
    }

private:
    class TacOpening {
    public:
        explicit TacOpening(std::function<BehaviorContext()> opener) : opener(std::move(opener)) {}

        std::function<BehaviorContext()> opener;
        BehaviorContext context;
    };

    std::shared_ptr<TacOpening> state;
};

class TacStores : public TacEntityRouter {
public:
    static constexpr const char* BUILT_IN = "indexeddb";

    static void keep(const std::string& entity, const TacLazyContext* context) {
        TacEntityRouter::current() = &router();
        opened()[entity].push_back(context);
    }

    static void release(const std::string& entity, const TacLazyContext* context) {
        std::vector<const TacLazyContext*>& kept = opened()[entity];
        kept.erase(std::remove(kept.begin(), kept.end(), context), kept.end());
        if (kept.empty()) {
            opened().erase(entity);
        }
    }

    std::optional<json::JsonNode> answer(const json::JsonNode& request) override {
        std::string adapter = request["adapter"].getValue<std::string>();
        const json::JsonNode& entity = request["command"]["entity"];
        bool named = entity.getValueKind() == json::JsonValueKind::String;
        const BehaviorContext* context = named ? keeping(entity.getValue<std::string>()) : holding(adapter);
        if (context == nullptr) {
            return std::nullopt;
        }
        bool own = context->adapters().entities.count(adapter) > 0;
        if (own || adapter != BUILT_IN) {
            return TacAdapters::entity(*context, request);
        }
        TacAdapters::allowed(*context, request);
        return std::nullopt;
    }

    bool keeps(const std::string& adapter) override {
        return holding(adapter) != nullptr;
    }

private:
    static TacStores& router() {
        static TacStores kept;
        return kept;
    }

    static std::map<std::string, std::vector<const TacLazyContext*>>& opened() {
        static std::map<std::string, std::vector<const TacLazyContext*>> contexts;
        return contexts;
    }

    static const BehaviorContext* keeping(const std::string& entity) {
        auto found = opened().find(entity);
        return found == opened().end() ? nullptr : found->second.back()->get();
    }

    static const BehaviorContext* holding(const std::string& adapter) {
        for (const auto& [entity, contexts] : opened()) {
            const BehaviorContext* context = contexts.back()->get();
            if (context->adapters().entities.count(adapter) > 0) {
                return context;
            }
        }
        return nullptr;
    }
};

class TacStoreContext {
public:
    TacStoreContext(std::string entity, TacLazyContext context) : entity(std::move(entity)), context(std::move(context)) {
        TacStores::keep(this->entity, &this->context);
    }

    ~TacStoreContext() {
        TacStores::release(entity, &context);
    }

    TacStoreContext(const TacStoreContext&) = delete;
    TacStoreContext& operator=(const TacStoreContext&) = delete;

private:
    std::string entity;
    TacLazyContext context;
};

class TacTaskBehaviors {
public:
    template <typename Result, typename Body>
    static Task<Result> run(const json::JsonNode& policy, std::vector<json::JsonNode> written, const BehaviorContext* context, Body body) {
        std::vector<json::JsonNode> arguments = TacBehaviors::prepared(policy, std::move(written));
        if (TacBehaviorData::present(policy["mutation"])) {
            co_return co_await mutate<Result>(policy, arguments, context, body);
        }
        if constexpr (!std::is_void_v<Result>) {
            if (TacBehaviorData::present(policy["keyArgs"])) {
                co_return co_await cached<Result>(policy, arguments, context, body);
            }
        }
        co_return co_await limited<Result>(policy, arguments, body, TacCall());
    }

private:
    using TacCacheCall = TacBehaviors::TacCacheCall;

    static std::set<std::string>& revalidating() {
        static std::set<std::string> keys;
        return keys;
    }

    template <typename Result, typename Body>
    static Task<Result> cached(const json::JsonNode& policy, const std::vector<json::JsonNode>& arguments, const BehaviorContext* context, Body& body) {
        const json::JsonNode& cache = policy["cache"];
        TacCacheCall call = TacBehaviors::cacheCall(policy, policy["name"], policy["keyArgs"], arguments, "cache:read");
        json::JsonNode entry = stored(context, TacBehaviors::request("cache.get", cache, call));
        if (TacBehaviors::fresh(entry, cache)) {
            co_return json::JsonCodec<Result>::fromJson(entry["value"]);
        }
        TacCall sent;
        sent.publicRead = policy["publicRead"].getValueKind() == json::JsonValueKind::True;
        sent.revalidate = revalidating().erase(call.key) > 0;
        Result value = co_await limited<Result>(policy, arguments, body, sent);
        write(context, cache, call, json::JsonCodec<Result>::toJson(value));
        co_return value;
    }

    template <typename Result, typename Body>
    static Task<Result> mutate(const json::JsonNode& policy, const std::vector<json::JsonNode>& arguments, const BehaviorContext* context, Body& body) {
        const json::JsonNode& mutation = policy["mutation"];
        const json::JsonNode& cache = policy["cache"];
        std::string operation = mutation["operation"].getValue<std::string>();
        TacCacheCall call = TacBehaviors::cacheCall(policy, mutation["method"], mutation["keyArgs"], TacBehaviors::mapped(mutation, arguments), operation);
        if constexpr (std::is_void_v<Result>) {
            co_await limited<Result>(policy, arguments, body, TacCall());
            forget(context, cache, call);
        } else {
            Result value = co_await limited<Result>(policy, arguments, body, TacCall());
            if (operation == "cache:put") {
                write(context, cache, call, json::JsonCodec<Result>::toJson(value));
            } else {
                forget(context, cache, call);
            }
            co_return value;
        }
    }

    template <typename Result, typename Body>
    static Task<Result> limited(const json::JsonNode& policy, const std::vector<json::JsonNode>& arguments, Body& body, TacCall sent) {
        const json::JsonNode& limit = policy["limit"];
        if (!TacBehaviorData::present(limit)) {
            co_return co_await attempt<Result>(policy, arguments, body, sent);
        }
        std::string key = policy["owner"].getValue<std::string>() + "." + policy["name"].getValue<std::string>();
        TacBehaviors::TacSlot slot(key, TacBehaviorData::integer(limit["limit"], 1, 1, 64));
        co_return co_await attempt<Result>(policy, arguments, body, sent);
    }

    template <typename Result, typename Body>
    static Task<Result> attempt(const json::JsonNode& policy, const std::vector<json::JsonNode>& arguments, Body& body, TacCall sent) {
        const json::JsonNode& retry = policy["retry"];
        if (!TacBehaviorData::present(retry)) {
            co_return co_await body(arguments, sent);
        }
        co_return co_await retried<Result>(retry, arguments, body, sent, 0);
    }

    template <typename Result, typename Body>
    static Task<Result> retried(const json::JsonNode& retry, const std::vector<json::JsonNode>& arguments, Body& body, TacCall sent, long long failures) {
        std::exception_ptr failure;
        bool retryable = true;
        try {
            co_return co_await body(arguments, sent);
        } catch (const TacCodedError& problem) {
            failure = std::current_exception();
            retryable = TacBehaviors::retryable(problem.code());
        } catch (const std::exception&) {
            failure = std::current_exception();
        }
        if (!retryable || failures == TacBehaviorData::integer(retry["maxRetries"], 1, 1, 8)) {
            std::rethrow_exception(failure);
        }
        TacHost::waitMilliseconds(TacBehaviors::backoff(retry, failures));
        co_return co_await retried<Result>(retry, arguments, body, sent, failures + 1);
    }

    static json::JsonNode stored(const BehaviorContext* context, const json::JsonNode& request) {
        if (context == nullptr) {
            return TacBehaviorData::ask(request);
        }
        if (context->adapters().caches.count(request["cache"].getValue<std::string>()) > 0) {
            return TacAdapters::cache(*context, request);
        }
        TacAdapters::allowed(*context, request);
        return TacBehaviorData::ask(request);
    }

    static void write(const BehaviorContext* context, const json::JsonNode& cache, const TacCacheCall& call, const json::JsonNode& value) {
        TacBehaviorData::bounded(value, static_cast<std::size_t>(cache["maxBytes"].getValue<double>()));
        json::JsonNode sent = TacBehaviors::request("cache.set", cache, call);
        sent["value"] = value;
        sent["expiresAt"] = TacHost::epochMilliseconds() + cache["ttlMs"].getValue<double>();
        sent["maxEntries"] = cache["maxEntries"];
        stored(context, sent);
    }

    static void forget(const BehaviorContext* context, const json::JsonNode& cache, const TacCacheCall& call) {
        stored(context, TacBehaviors::request("cache.delete", cache, call));
        revalidating().insert(call.key);
    }
};

}

using tachyon::TacLazyContext;
