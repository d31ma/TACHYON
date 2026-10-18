namespace tachyon::yon {

class YonApplication {
public:
    static json::JsonNode snapshot() {
        const char* written = std::getenv("YON_APPLICATION_JSON");
        if (written == nullptr) {
            throw TacCodedError("The Yon application snapshot is missing.", "configuration");
        }
        return json::JsonNode::parse(written);
    }
};

}
