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
        } else {
            Result result = call(service, json::JsonSerializer::deserialize<std::decay_t<Arguments>>(values[Index])...);
            return json::JsonSerializer::serializeToNode(result);
        }
    }
};

}
