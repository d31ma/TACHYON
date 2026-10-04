namespace tachyon::json {

class JsonSerializer {
public:
    template <typename T>
    static std::string serialize(const T& value) {
        return JsonCodec<T>::toJson(value).toJsonString();
    }

    template <typename T>
    static JsonNode serializeToNode(const T& value) {
        return JsonCodec<T>::toJson(value);
    }

    template <typename T>
    static T deserialize(std::string_view json) {
        return JsonCodec<T>::fromJson(JsonNode::parse(json));
    }

    template <typename T>
    static T deserialize(const std::string& json) {
        return deserialize<T>(std::string_view(json));
    }

    template <typename T>
    static T deserialize(const char* json) {
        return deserialize<T>(std::string_view(json));
    }

    template <typename T>
    static T deserialize(const JsonNode& node) {
        return JsonCodec<T>::fromJson(node);
    }
};

}
