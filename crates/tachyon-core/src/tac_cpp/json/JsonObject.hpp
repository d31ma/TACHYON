namespace tachyon::json {

class JsonObject : public JsonNode {
public:
    JsonObject() : JsonNode(JsonValueKind::Object) {}
};

}
