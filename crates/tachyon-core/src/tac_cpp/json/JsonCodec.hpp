namespace tachyon::json {

template <typename T>
struct JsonUnknownType : std::false_type {};

template <typename T, typename Enable = void>
struct JsonCodec {
    static_assert(JsonUnknownType<T>::value, "Tachyon has no JSON codec for this type. Use bool, a number, std::string, std::vector, std::optional, std::map with std::string keys, JsonNode or a model from client/models.");
};

template <>
struct JsonCodec<bool> {
    static JsonNode toJson(bool value) { return JsonNode(value); }
    static bool fromJson(const JsonNode& node) { return node.getValue<bool>(); }
};

template <typename T>
struct JsonCodec<T, std::enable_if_t<std::is_arithmetic_v<T> && !std::is_same_v<T, bool>>> {
    static JsonNode toJson(T value) { return JsonNode(value); }
    static T fromJson(const JsonNode& node) { return node.getValue<T>(); }
};

template <>
struct JsonCodec<std::string> {
    static JsonNode toJson(const std::string& value) { return JsonNode(value); }
    static std::string fromJson(const JsonNode& node) { return node.getValue<std::string>(); }
};

template <>
struct JsonCodec<JsonNode> {
    static JsonNode toJson(const JsonNode& value) { return value; }
    static JsonNode fromJson(const JsonNode& node) { return node; }
};

template <typename T>
struct JsonCodec<std::vector<T>> {
    static JsonNode toJson(const std::vector<T>& value) {
        JsonArray array;
        for (const T& item : value) {
            array.add(JsonCodec<T>::toJson(item));
        }
        return array;
    }

    static std::vector<T> fromJson(const JsonNode& node) {
        if (node.getValueKind() != JsonValueKind::Array) {
            throw JsonException("The JSON value is not an array.");
        }
        std::vector<T> value;
        for (const JsonNode& item : node.items()) {
            value.push_back(JsonCodec<T>::fromJson(item));
        }
        return value;
    }
};

template <typename T>
struct JsonCodec<std::optional<T>> {
    static JsonNode toJson(const std::optional<T>& value) {
        if (!value.has_value()) {
            return JsonNode();
        }
        return JsonCodec<T>::toJson(*value);
    }

    static std::optional<T> fromJson(const JsonNode& node) {
        if (node.isNull()) {
            return std::nullopt;
        }
        return JsonCodec<T>::fromJson(node);
    }
};

template <typename T>
struct JsonCodec<std::map<std::string, T>> {
    static JsonNode toJson(const std::map<std::string, T>& value) {
        JsonObject object;
        for (const auto& [key, item] : value) {
            object[key] = JsonCodec<T>::toJson(item);
        }
        return object;
    }

    static std::map<std::string, T> fromJson(const JsonNode& node) {
        if (node.getValueKind() != JsonValueKind::Object) {
            throw JsonException("The JSON value is not an object.");
        }
        std::map<std::string, T> value;
        for (const std::string& key : node.propertyNames()) {
            value[key] = JsonCodec<T>::fromJson(node[key]);
        }
        return value;
    }
};

}
