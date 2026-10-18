namespace tachyon::yon {

class YonGateway {
public:
    static constexpr const char* FAILED = "Gateway call failed.";

    static json::JsonNode call(const std::string& key, const std::vector<json::JsonNode>& arguments) {
        json::JsonNode answer = ask(key, arguments);
        bool settled = answer.getValueKind() == json::JsonValueKind::Object && answer.containsKey("result") && !answer.containsKey("error");
        if (!settled) {
            throw TacCodedError(FAILED, code(answer));
        }
        return answer["result"];
    }

private:
    static json::JsonNode ask(const std::string& key, const std::vector<json::JsonNode>& arguments) {
        if (!YonFrames::enabled()) {
            return failure("adapter");
        }
        json::JsonArray written;
        for (const json::JsonNode& argument : arguments) {
            written.add(argument);
        }
        json::JsonObject command;
        command["operation"] = "gateway";
        command["key"] = key;
        command["arguments"] = written;
        json::JsonObject asked;
        asked["kind"] = "resource";
        asked["command"] = command;
        YonFrames::write(asked.toJsonString());
        std::string raw;
        if (!YonFrames::read(raw)) {
            return failure("adapter");
        }
        try {
            return json::JsonNode::parse(raw);
        } catch (const std::exception&) {
            return failure("adapter");
        }
    }

    static json::JsonNode failure(const std::string& kind) {
        json::JsonObject failed;
        failed["error"] = kind;
        return failed;
    }

    static std::string code(const json::JsonNode& answer) {
        bool named = answer.getValueKind() == json::JsonValueKind::Object && answer.containsKey("error") && answer["error"].getValueKind() == json::JsonValueKind::String;
        std::string written = named ? answer["error"].getValue<std::string>() : std::string();
        bool known = written == "invalid" || written == "timeout" || written == "capacity";
        return known ? written : "adapter";
    }
};

}
