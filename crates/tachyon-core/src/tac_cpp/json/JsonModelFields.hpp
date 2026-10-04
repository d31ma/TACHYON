namespace tachyon::json {

class JsonModelFields {
public:
    template <typename T>
    static void write(JsonNode& object, const char* key, const T& value) {
        object[key] = JsonCodec<T>::toJson(value);
    }

    template <typename T>
    static void read(const JsonNode& object, const char* key, T& field) {
        if (object.containsKey(key)) {
            field = JsonCodec<T>::fromJson(object[key]);
        }
    }

    static void requireObject(const JsonNode& node, std::string_view model) {
        if (node.getValueKind() != JsonValueKind::Object) {
            throw JsonException("The JSON value for " + std::string(model) + " is not an object.");
        }
    }

    static std::string enumName(const JsonNode& node, std::string_view model) {
        if (node.getValueKind() != JsonValueKind::String) {
            throw JsonException("The JSON value for " + std::string(model) + " is not a name.");
        }
        return node.getValue<std::string>();
    }

    [[noreturn]] static void unknownName(std::string_view model) {
        throw JsonException("The JSON text is not a " + std::string(model) + " name.");
    }
};

}
