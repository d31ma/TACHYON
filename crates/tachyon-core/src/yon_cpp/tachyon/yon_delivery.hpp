namespace tachyon::yon {

class YonDelivery {
public:
    static constexpr const char* OPERATION = "event.deliver";

    static void receive(const json::JsonNode& fields) {
        try {
            if (fields.getValueKind() != json::JsonValueKind::Object || !fields.containsKey("operation") || fields["operation"].getValueKind() != json::JsonValueKind::String) {
                return;
            }
            if (fields["operation"].getValue<std::string>() != OPERATION) {
                return;
            }
            json::JsonNode delivered = json::JsonNode::parse(fields["body"]["data"].getValue<std::string>());
            std::vector<std::string> keys;
            for (const json::JsonNode& key : delivered["listeners"].items()) {
                keys.push_back(key.getValue<std::string>());
            }
            current().topic = delivered["topic"].getValue<std::string>();
            current().payload = delivered["payload"];
            current().pending = true;
            YonEvents::current().only(std::move(keys));
        } catch (const std::exception&) {
            YonEvents::report("Yon topic delivery was not readable.");
        }
    }

    static void deliver() {
        if (current().pending) {
            current().pending = false;
            current().delivered = true;
            YonEvents::current().receive(current().topic, current().payload);
        }
    }

    static YonAnswer unrouted() {
        return YonAnswer::empty(current().delivered ? 204 : 405);
    }

    static void reset() {
        current() = YonDelivery();
    }

private:
    std::string topic;
    json::JsonNode payload;
    bool pending = false;
    bool delivered = false;

    static YonDelivery& current() {
        static YonDelivery delivery;
        return delivery;
    }
};

}
