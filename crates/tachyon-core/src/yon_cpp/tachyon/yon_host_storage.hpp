namespace tachyon {

class YonHostStorage {
public:
    class Storage {
    public:
        virtual ~Storage() = default;
        virtual json::JsonNode answer(const json::JsonNode& request) = 0;
        virtual json::JsonNode partition() = 0;
    };

    static Storage*& current() {
        static Storage* running = nullptr;
        return running;
    }

    static Storage*& stores() {
        static Storage* kept = nullptr;
        return kept;
    }

    static std::string storage(const std::string& text) {
        json::JsonNode request = json::JsonNode::parse(text);
        Storage* running = stored(request) ? stores() : current();
        if (running == nullptr) {
            throw TacCodedError("This Yon C++ call has no behavior context. A class whose methods carry [[Cached]], [[UpdateCache]], [[ClearCache]] or [[AllOrNothing]] declares `BehaviorContext behaviors;` and opens it from a Setup (ADR 0099).", "invalid");
        }
        json::JsonObject answer;
        answer["value"] = running->answer(request);
        return answer.toJsonString();
    }

    static json::JsonNode partition(const json::JsonNode& written) {
        Storage* running = current();
        return running == nullptr ? written : running->partition();
    }

    static bool stored(const json::JsonNode& request) {
        bool entity = request["op"].getValueKind() == json::JsonValueKind::String && request["op"].getValue<std::string>() == "entity";
        return entity && request["command"]["entity"].getValueKind() == json::JsonValueKind::String;
    }
};

}
