namespace tachyon {

class TacPublishedEvent {
public:
    std::string name;
    json::JsonNode value;
    bool retain = false;
};

class TacEventSink {
public:
    static void publish(const std::string& name, const json::JsonNode& value, bool retain) {
        published().push_back(TacPublishedEvent{name, value, retain});
    }

    static std::vector<TacPublishedEvent>& published() {
        static std::vector<TacPublishedEvent> events;
        return events;
    }
};

}
