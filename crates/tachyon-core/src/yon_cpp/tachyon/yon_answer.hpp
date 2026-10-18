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

    YonAnswer(int status, std::string body, bool hasJson) : status(status), body(std::move(body)), hasJson(hasJson) {}
};

}
