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
