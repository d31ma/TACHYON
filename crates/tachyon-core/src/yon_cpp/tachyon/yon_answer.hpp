namespace tachyon::yon {

class YonAnswer {
public:
    static YonAnswer empty(int status) {
        return YonAnswer(status, std::string(), false);
    }

    static YonAnswer json(int status, std::string body) {
        return YonAnswer(status, std::move(body), true);
    }

    static YonAnswer failure(int status, const std::string& message) {
        json::JsonObject body;
        body["error"] = message;
        return json(status, body.toJsonString());
    }

    static YonAnswer rejected(const std::vector<TacCheckFailure>& failures) {
        YonAnswer answer(422, std::string(), false);
        for (const TacCheckFailure& failure : failures) {
            json::JsonObject violation;
            violation["field"] = failure.field;
            violation["rule"] = ruleName(failure.rule);
            answer.violations.add(violation);
        }
        return answer;
    }

    template <typename T>
    static YonAnswer of(const T& value) {
        json::JsonNode node = json::JsonSerializer::serializeToNode(value);
        if (node.isNull()) {
            return empty(204);
        }
        return json(200, node.toJsonString());
    }

    std::string write() const {
        json::JsonObject answer;
        answer["status"] = status;
        if (violations.count() > 0) {
            answer["violations"] = violations;
            return answer.toJsonString();
        }
        json::JsonObject headers;
        if (hasJson) {
            json::JsonArray types;
            types.add("application/json");
            headers["content-type"] = types;
        }
        answer["headers"] = headers;
        answer["body"] = body;
        return answer.toJsonString();
    }

private:
    int status;
    std::string body;
    bool hasJson;
    json::JsonArray violations;

    YonAnswer(int status, std::string body, bool hasJson) : status(status), body(std::move(body)), hasJson(hasJson) {}

    static std::string ruleName(const std::string& rule) {
        static const std::map<std::string, std::string> names = {
            {"notNull", "Required"},
            {"notBlank", "NotEmpty"},
            {"size", "Length"},
            {"min", "Min"},
            {"max", "Max"},
        };
        std::map<std::string, std::string>::const_iterator found = names.find(rule);
        return found == names.end() ? rule : found->second;
    }
};

}
