namespace tachyon::json {

class JsonArray : public JsonNode {
public:
    JsonArray() : JsonNode(JsonValueKind::Array) {}
};

}
