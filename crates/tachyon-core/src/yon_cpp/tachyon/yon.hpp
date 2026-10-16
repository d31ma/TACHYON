namespace tachyon::yon {

class YonArgumentException : public std::runtime_error {
public:
    explicit YonArgumentException(const std::string& message) : std::runtime_error(message) {}
};

}

namespace tachyon::yon {

class YonFrame {
public:
    explicit YonFrame(json::JsonNode fields) : fields(std::move(fields)) {}

    std::string text(const std::string& key) const {
        const json::JsonNode& value = field(key);
        if (value.getValueKind() != json::JsonValueKind::String) {
            return std::string();
        }
        return value.getValue<std::string>();
    }

    template <typename T>
    T argument(std::size_t index) const {
        try {
            return json::JsonSerializer::deserialize<T>(argumentNode(index));
        } catch (const json::JsonException& error) {
            throw YonArgumentException("Argument " + std::to_string(index + 1) + " does not match its parameter type: " + error.what());
        }
    }

private:
    json::JsonNode fields;

    const json::JsonNode& field(const std::string& key) const {
        static const json::JsonNode nothing;
        if (fields.getValueKind() != json::JsonValueKind::Object || !fields.containsKey(key)) {
            return nothing;
        }
        return fields[key];
    }

    json::JsonNode argumentNode(std::size_t index) const {
        const json::JsonNode& values = field("arguments");
        if (values.getValueKind() != json::JsonValueKind::Array || index >= values.count()) {
            return json::JsonNode(nullptr);
        }
        return values[index];
    }
};

}

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

namespace tachyon::yon {

class YonMain {
public:
    static constexpr std::size_t MAX_FRAME_BYTES = 16 * 1024 * 1024;
    static constexpr int PROTOCOL_FAILURE = 70;

    template <typename Routes>
    static int run() {
        std::string raw;
        if (!readInput(raw)) {
            return PROTOCOL_FAILURE;
        }
        std::optional<json::JsonNode> fields = parsed(raw);
        if (!fields.has_value()) {
            return PROTOCOL_FAILURE;
        }
        YonFrame frame(std::move(*fields));
        std::string written = answered<Routes>(frame).write();
        std::fwrite(written.data(), 1, written.size(), stdout);
        std::fflush(stdout);
        return 0;
    }

private:
    static bool readInput(std::string& raw) {
        char buffer[65536];
        std::size_t count = std::fread(buffer, 1, sizeof buffer, stdin);
        while (count > 0) {
            raw.append(buffer, count);
            if (raw.size() > MAX_FRAME_BYTES) {
                return false;
            }
            count = std::fread(buffer, 1, sizeof buffer, stdin);
        }
        return true;
    }

    static std::optional<json::JsonNode> parsed(const std::string& raw) {
        try {
            return json::JsonReader::read(raw, MAX_FRAME_BYTES);
        } catch (const json::JsonException&) {
            return std::nullopt;
        }
    }

    template <typename Routes>
    static YonAnswer answered(const YonFrame& frame) {
        try {
            return Routes::answer(frame);
        } catch (const YonArgumentException& error) {
            return YonAnswer::failure(400, error.what());
        } catch (const std::exception& error) {
            report(frame, error.what());
        } catch (...) {
            report(frame, "The route method threw a value that is not a std::exception.");
        }
        return YonAnswer::failure(500, "Handler failed.");
    }

    static void report(const YonFrame& frame, const std::string& message) {
        json::JsonObject line;
        line["event"] = "handler.cpp_exception";
        line["request_id"] = frame.text("request_id");
        line["message"] = message.substr(0, 2048);
        std::string written = line.toJsonString() + "\n";
        std::fwrite(written.data(), 1, written.size(), stderr);
    }
};

}
