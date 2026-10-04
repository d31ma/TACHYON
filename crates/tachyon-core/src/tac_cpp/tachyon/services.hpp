namespace tachyon {

class ApplicationEventPublisher {
public:
    template <typename Model>
    const Model& publishEvent(const Model& value, bool retain = false) const {
        std::string name(json::JsonCodec<Model>::NAME);
        TacEventSink::publish(name, json::JsonSerializer::serializeToNode(value), retain);
        return value;
    }
};

}

namespace tachyon {

class TacNeeds {
public:
    template <typename Provider>
    static Provider& of() {
        static Provider provider;
        return provider;
    }
};

}

using tachyon::ApplicationEventPublisher;
using tachyon::Task;
