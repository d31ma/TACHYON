namespace tachyon::yon {

class YonAnswer {
public:
    static YonAnswer empty(int status) {
        return YonAnswer(status, std::string(), false);
    }

    static YonAnswer json(int status, const std::string& body) {
        return YonAnswer(status, YonText::repaired(body), true);
    }

    static YonAnswer failure(int status, const std::string& message) {
        json::JsonObject body;
        body["error"] = message;
        return json(status, body.toJsonString());
    }

    static YonAnswer streamed() {
        YonAnswer answer(200, std::string(), false);
        answer.alreadySent = true;
        return answer;
    }

    static YonAnswer failedStream() {
        YonAnswer answer = streamed();
        answer.streamBroken = true;
        return answer;
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

    static YonAnswer of(const http::HttpResponseMessage& message) {
        int status = static_cast<int>(message.statusCode);
        std::string body = status == 204 ? std::string() : message.content.readAsString();
        YonAnswer answer(status, body, !body.empty() && !message.headers.get("content-type").has_value());
        for (const auto& [name, value] : message.headers.all()) {
            answer.headers[lowered(name)] = value;
        }
        return answer;
    }

    static YonAnswer of(const std::optional<http::HttpResponseMessage>& message) {
        if (!message.has_value()) {
            return empty(204);
        }
        return of(*message);
    }

    template <typename T>
    static YonAnswer handled(const T& value, int status) {
        return json(status, json::JsonSerializer::serializeToNode(value).toJsonString());
    }

    static YonAnswer handled(const http::HttpResponseMessage& message, int status) {
        YonAnswer answer = of(message);
        answer.status = answer.status == 200 ? status : answer.status;
        return answer;
    }

    bool isStreamed() const {
        return alreadySent;
    }

    bool isFailedStream() const {
        return streamBroken;
    }

    std::string write(const json::JsonNode& topics) const {
        json::JsonObject answer;
        answer["status"] = status;
        if (violations.count() > 0) {
            answer["violations"] = violations;
            return YonText::repaired(answer.toJsonString());
        }
        json::JsonObject written;
        if (hasJson) {
            json::JsonArray types;
            types.add("application/json");
            written["content-type"] = types;
        }
        for (const auto& [name, value] : headers) {
            json::JsonArray values;
            values.add(value);
            written[name] = values;
        }
        answer["headers"] = written;
        answer["body"] = YonText::valid(body) ? json::JsonNode(body) : encoded(body);
        answer["topics"] = topics;
        return YonText::repaired(answer.toJsonString());
    }

private:
    int status;
    std::string body;
    bool hasJson;
    json::JsonArray violations;
    std::map<std::string, std::string> headers;
    bool alreadySent = false;
    bool streamBroken = false;

    YonAnswer(int status, std::string body, bool hasJson) : status(status), body(std::move(body)), hasJson(hasJson) {}

    static json::JsonNode encoded(const std::string& bytes) {
        json::JsonObject body;
        body["encoding"] = "base64";
        body["data"] = YonText::base64(bytes);
        return body;
    }

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

    static std::string lowered(std::string name) {
        std::transform(name.begin(), name.end(), name.begin(), [](unsigned char character) {
            return static_cast<char>(std::tolower(character));
        });
        return name;
    }
};

}
