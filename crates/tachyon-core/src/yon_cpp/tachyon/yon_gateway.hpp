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

namespace tachyon::yon {

class YonGatewayCall {
public:
    explicit YonGatewayCall(const YonFrame& frame) : fields(read(frame)) {}

    std::string method() const {
        bool named = fields.getValueKind() == json::JsonValueKind::Object && fields.containsKey("method") && fields["method"].getValueKind() == json::JsonValueKind::String;
        return named ? fields["method"].getValue<std::string>() : std::string();
    }

    json::JsonNode argument(std::size_t index) const {
        bool listed = fields.getValueKind() == json::JsonValueKind::Object && fields.containsKey("arguments") && fields["arguments"].getValueKind() == json::JsonValueKind::Array;
        if (!listed || index >= fields["arguments"].count()) {
            return json::JsonNode(nullptr);
        }
        return fields["arguments"][index];
    }

    static YonAnswer answer(const json::JsonNode& result) {
        json::JsonObject written;
        written["result"] = result;
        return YonAnswer::json(200, written.toJsonString());
    }

private:
    json::JsonNode fields;

    static json::JsonNode read(const YonFrame& frame) {
        const json::JsonNode& body = frame.field("body");
        bool written = body.getValueKind() == json::JsonValueKind::Object && body.containsKey("data") && body["data"].getValueKind() == json::JsonValueKind::String;
        if (!written) {
            return json::JsonNode(nullptr);
        }
        try {
            return json::JsonNode::parse(body["data"].getValue<std::string>());
        } catch (const std::exception&) {
            return json::JsonNode(nullptr);
        }
    }
};

}
