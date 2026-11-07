namespace tachyon {

class TacTasks {
public:
    template <typename Value>
    static std::uint32_t start(Task<Value> task) {
        TacTasks& tasks = current();
        tasks.last = std::max<std::uint32_t>(tasks.last + 1, 1);
        std::uint32_t id = tasks.last;
        report<Value>(id, TacTaskOwner::current(), std::move(task));
        return id;
    }

private:
    std::uint32_t last = 0;

    static TacTasks& current() {
        static TacTasks tasks;
        return tasks;
    }

    template <typename Value>
    static TacDetached report(std::uint32_t id, std::string owner, Task<Value> task) {
        std::string answer;
        try {
            answer = co_await settled<Value>(std::move(task));
        } catch (const std::exception& problem) {
            answer = failure(problem);
        }
        TacTaskOwner::finish(id, owner, answer);
    }

    template <typename Value>
    static Task<std::string> settled(Task<Value> task) {
        json::JsonObject done;
        if constexpr (std::is_void_v<Value>) {
            co_await task;
            done["value"] = json::JsonNode();
        } else {
            Value value = co_await task;
            done["value"] = json::JsonSerializer::serializeToNode(value);
        }
        co_return done.toJsonString();
    }

    static std::string failure(const std::exception& problem) {
        json::JsonObject failed;
        failed["error"] = std::string(problem.what());
        const TacCodedError* coded = dynamic_cast<const TacCodedError*>(&problem);
        if (coded != nullptr) {
            failed["code"] = coded->code();
        }
        return failed.toJsonString();
    }
};

}

namespace tachyon {

class TacBind {
public:
    template <typename Service, typename Field>
    static TacMember field(std::string name, Field Service::*member) {
        TacMember bound;
        bound.name = std::move(name);
        bound.read = [member](void* instance) {
            Service* service = static_cast<Service*>(instance);
            return json::JsonSerializer::serializeToNode(service->*member);
        };
        bound.write = [member, label = bound.name](void* instance, const json::JsonNode& value) {
            if constexpr (std::is_const_v<Field>) {
                throw std::runtime_error("Companion field is read-only: " + label);
            } else {
                Service* service = static_cast<Service*>(instance);
                service->*member = json::JsonSerializer::deserialize<Field>(value);
            }
        };
        return bound;
    }

    template <typename Service, typename Result, typename... Arguments>
    static TacMethod method(std::string name, Result (Service::*function)(Arguments...)) {
        return bindMethod<Service, Result, Arguments...>(std::move(name), [function](Service& service, Arguments... values) -> Result {
            return (service.*function)(std::forward<Arguments>(values)...);
        });
    }

    template <typename Service, typename Result, typename... Arguments>
    static TacMethod method(std::string name, Result (Service::*function)(Arguments...) const) {
        return bindMethod<Service, Result, Arguments...>(std::move(name), [function](Service& service, Arguments... values) -> Result {
            return (service.*function)(std::forward<Arguments>(values)...);
        });
    }

private:
    template <typename Service, typename Result, typename... Arguments, typename Call>
    static TacMethod bindMethod(std::string name, Call call) {
        TacMethod bound;
        bound.name = name;
        bound.asynchronous = TacIsTask<Result>::value;
        bound.call = [call, name](void* instance, const json::JsonNode& values) {
            std::size_t given = values.getValueKind() == json::JsonValueKind::Array ? values.count() : 0;
            if (given != sizeof...(Arguments)) {
                throw std::runtime_error("Companion method " + name + " takes " + std::to_string(sizeof...(Arguments)) + " arguments.");
            }
            Service& service = *static_cast<Service*>(instance);
            return invoke<Service, Result, Arguments...>(service, call, values, std::index_sequence_for<Arguments...>{});
        };
        return bound;
    }

    template <typename Service, typename Result, typename... Arguments, typename Call, std::size_t... Index>
    static json::JsonNode invoke(Service& service, Call& call, const json::JsonNode& values, std::index_sequence<Index...>) {
        if constexpr (std::is_void_v<Result>) {
            call(service, json::JsonSerializer::deserialize<std::decay_t<Arguments>>(values[Index])...);
            return json::JsonNode();
        } else if constexpr (TacIsTask<Result>::value) {
            Result task = call(service, json::JsonSerializer::deserialize<std::decay_t<Arguments>>(values[Index])...);
            return json::JsonNode(TacTasks::start(std::move(task)));
        } else {
            Result result = call(service, json::JsonSerializer::deserialize<std::decay_t<Arguments>>(values[Index])...);
            return json::JsonSerializer::serializeToNode(result);
        }
    }
};

}

namespace tachyon {

class TacEventSink {
public:
    static void publish(const std::string& name, const json::JsonNode& value, bool retain) {
        TacPage::publish(name, value, retain);
    }
};

}
