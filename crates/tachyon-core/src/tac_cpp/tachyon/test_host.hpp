namespace tachyon {

class TacHost {
public:
    static constexpr const char* REALM = "tac";

    static json::JsonNode partition(const json::JsonNode& written) {
        return written;
    }

    static std::string storage(const std::string& text) {
        json::JsonNode request = json::JsonNode::parse(text);
        std::string operation = request["op"].getValueKind() == json::JsonValueKind::String ? request["op"].getValue<std::string>() : std::string();
        if (operation == "entity") {
            return entity(request["command"]);
        }
        if (operation.rfind("cache.", 0) != 0) {
            return missing();
        }
        std::string key = request["cache"].toJsonString() + request["key"].toJsonString();
        json::JsonObject answer;
        if (operation == "cache.get") {
            json::JsonObject entry;
            auto found = cached().find(key);
            entry["hit"] = found != cached().end();
            if (found != cached().end()) {
                entry["value"] = found->second["value"];
                entry["expiresAt"] = found->second["expiresAt"];
            }
            answer["value"] = entry;
        } else if (operation == "cache.set") {
            cached()[key] = request;
            answer["value"] = nullptr;
        } else {
            cached().erase(key);
            answer["value"] = nullptr;
        }
        return answer.toJsonString();
    }

    static void waitMilliseconds(double) {}

    static void reset() {
        cached().clear();
        fakes().clear();
        TacMemoryEntities::reset();
    }

    static std::string web(const std::string& text) {
        json::JsonNode request = json::JsonNode::parse(text);
        auto found = fakes().find(request["api"].getValue<std::string>() + "." + request["call"].getValue<std::string>());
        if (found == fakes().end()) {
            return unanswered();
        }
        json::JsonObject answer;
        answer["value"] = found->second;
        return answer.toJsonString();
    }

    static void fake(const std::string& key, json::JsonNode value) {
        fakes()[key] = std::move(value);
    }

    static double epochMilliseconds() {
        return 0;
    }

    static void httpStart(std::uint32_t, const std::string&) {}

    static void httpCancel(std::uint32_t) {}

    static void socketStart(std::uint32_t, const std::string&, const std::string&) {}

private:
    static std::string entity(const json::JsonNode& command) {
        json::JsonObject answer;
        try {
            answer["value"] = TacMemoryEntities::execute(command);
        } catch (const std::exception& problem) {
            answer["error"] = problem.what();
        }
        return answer.toJsonString();
    }

    static std::map<std::string, json::JsonNode>& cached() {
        static std::map<std::string, json::JsonNode> entries;
        return entries;
    }

    static std::map<std::string, json::JsonNode>& fakes() {
        static std::map<std::string, json::JsonNode> answers;
        return answers;
    }

    static std::string missing() {
        json::JsonObject failure;
        failure["error"] = "C++ test units keep only cache and entity storage (ADR 0099).";
        return failure.toJsonString();
    }

    static std::string unanswered() {
        json::JsonObject failure;
        failure["error"] = "This browser API needs the page. A test unit has no page: give the call an answer with BrowserFakes::answer in the test unit.";
        return failure.toJsonString();
    }
};

class BrowserFakes {
public:
    template <typename Value>
    static void answer(const std::string& api, const std::string& call, const Value& value) {
        if constexpr (std::is_constructible_v<json::JsonNode, const Value&>) {
            TacHost::fake(api + "." + call, json::JsonNode(value));
        } else {
            TacHost::fake(api + "." + call, json::JsonSerializer::serializeToNode(value));
        }
    }
};

}

using tachyon::BrowserFakes;

namespace tachyon {

class TacTaskOwner {
public:
    static std::string current() {
        return std::string();
    }

    static void resumeFor(const std::string&, std::coroutine_handle<> waiting) {
        waiting.resume();
    }

    static void finish(std::uint32_t, const std::string&, const std::string&) {}
};

}
