namespace tachyon {

class TacEventSink {
public:
    static void publish(const std::string& name, const json::JsonNode& value, bool retain) {
        TacPage::publish(name, value, retain);
    }
};

}
