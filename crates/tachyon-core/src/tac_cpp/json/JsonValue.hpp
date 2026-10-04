namespace tachyon::json {

class JsonValue : public JsonNode {
public:
    template <typename T>
    static JsonNode create(T value) {
        return JsonNode(value);
    }
};

}
