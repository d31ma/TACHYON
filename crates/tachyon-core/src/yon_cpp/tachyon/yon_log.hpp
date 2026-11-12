namespace tachyon::yon {

class YonLog {
public:
    using Fields = std::initializer_list<std::pair<const std::string, json::JsonNode>>;

    static void debug(const std::string& message, Fields fields = {}) { write("debug", message, fields); }
    static void info(const std::string& message, Fields fields = {}) { write("info", message, fields); }
    static void warn(const std::string& message, Fields fields = {}) { write("warn", message, fields); }
    static void error(const std::string& message, Fields fields = {}) { write("error", message, fields); }

private:
    static void write(const char* level, const std::string& message, Fields fields) {
        json::JsonObject values;
        for (const auto& field : fields) {
            values[field.first] = field.second;
        }
        json::JsonObject line;
        line["event"] = "yon.log";
        line["level"] = level;
        line["message"] = message;
        line["fields"] = values;
        std::string written = line.toJsonString() + "\n";
        std::fwrite(written.data(), 1, written.size(), stderr);
        std::fflush(stderr);
    }
};

}
