namespace tachyon::yon {

class YonCondition {
public:
    static bool test(const json::JsonNode& node, const json::JsonNode& value, const std::string& event) {
        std::string kind = word(node, "k");
        if (kind == "and") {
            return test(field(node, "l"), value, event) && test(field(node, "r"), value, event);
        }
        if (kind == "or") {
            return test(field(node, "l"), value, event) || test(field(node, "r"), value, event);
        }
        if (kind == "not") {
            return !test(field(node, "e"), value, event);
        }
        if (kind == "truthy") {
            return truthy(operand(field(node, "e"), value, event));
        }
        if (kind == "cmp") {
            return compare(word(node, "op"), operand(field(node, "l"), value, event), operand(field(node, "r"), value, event));
        }
        return false;
    }

private:
    static const json::JsonNode& field(const json::JsonNode& node, const std::string& key) {
        static const json::JsonNode nothing;
        if (node.getValueKind() != json::JsonValueKind::Object || !node.containsKey(key)) {
            return nothing;
        }
        return node[key];
    }

    static std::string word(const json::JsonNode& node, const std::string& key) {
        const json::JsonNode& found = field(node, key);
        if (found.getValueKind() != json::JsonValueKind::String) {
            return std::string();
        }
        return found.getValue<std::string>();
    }

    static json::JsonNode operand(const json::JsonNode& node, const json::JsonNode& value, const std::string& event) {
        std::string kind = word(node, "k");
        if (kind == "lit") {
            return field(node, "v");
        }
        if (kind == "event") {
            return json::JsonNode(event);
        }
        json::JsonNode current = value;
        for (const json::JsonNode& segment : field(node, "path").items()) {
            current = step(current, segment);
        }
        return current;
    }

    static json::JsonNode step(const json::JsonNode& current, const json::JsonNode& segment) {
        if (current.getValueKind() == json::JsonValueKind::Object && segment.getValueKind() == json::JsonValueKind::String) {
            return field(current, segment.getValue<std::string>());
        }
        if (current.getValueKind() != json::JsonValueKind::Array || segment.getValueKind() != json::JsonValueKind::Number) {
            return json::JsonNode(nullptr);
        }
        double index = segment.getValue<double>();
        bool inside = index >= 0 && std::floor(index) == index && index < static_cast<double>(current.count());
        return inside ? current[static_cast<std::size_t>(index)] : json::JsonNode(nullptr);
    }

    static bool truthy(const json::JsonNode& value) {
        switch (value.getValueKind()) {
        case json::JsonValueKind::Null:
        case json::JsonValueKind::Undefined:
        case json::JsonValueKind::False:
            return false;
        case json::JsonValueKind::String:
            return !value.getValue<std::string>().empty();
        case json::JsonValueKind::Number:
            return value.getValue<double>() != 0;
        default:
            return true;
        }
    }

    static bool compare(const std::string& op, const json::JsonNode& left, const json::JsonNode& right) {
        if (op == "eq" || op == "ne") {
            return (op == "eq") == equal(left, right);
        }
        int order = 0;
        if (!ordered(left, right, order)) {
            return false;
        }
        if (op == "lt") {
            return order < 0;
        }
        if (op == "le") {
            return order <= 0;
        }
        if (op == "gt") {
            return order > 0;
        }
        return order >= 0;
    }

    static bool equal(const json::JsonNode& left, const json::JsonNode& right) {
        bool sameKind = left.getValueKind() == right.getValueKind();
        bool plain = left.getValueKind() != json::JsonValueKind::Object && left.getValueKind() != json::JsonValueKind::Array;
        return sameKind && plain && json::JsonNode::deepEquals(left, right);
    }

    static bool ordered(const json::JsonNode& left, const json::JsonNode& right, int& order) {
        json::JsonValueKind kind = left.getValueKind();
        if (kind != right.getValueKind()) {
            return false;
        }
        if (kind == json::JsonValueKind::Number) {
            double first = left.getValue<double>();
            double second = right.getValue<double>();
            order = first < second ? -1 : (first > second ? 1 : 0);
            return true;
        }
        if (kind == json::JsonValueKind::String) {
            order = left.getValue<std::string>().compare(right.getValue<std::string>());
            return true;
        }
        return false;
    }
};

class YonTopics {
public:
    static const std::vector<std::string>& names();

    static bool named(const std::string& name) {
        const std::vector<std::string>& topics = names();
        return std::find(topics.begin(), topics.end(), name) != topics.end();
    }
};

class YonPublished {
public:
    std::string name;
    std::string encoded;
};

class YonListenerEntry {
public:
    std::string name;
    long long order = 0;
    json::JsonNode condition;
    bool background = false;
    std::string returns;
    std::size_t sequence = 0;
    std::function<json::JsonNode(const json::JsonNode&)> callback;
};

class YonEvents {
public:
    static constexpr std::size_t MAX_QUEUE = 128;
    static constexpr std::size_t MAX_DELIVERIES = 4096;
    static constexpr std::size_t MAX_LISTENERS = 1024;
    static constexpr std::size_t MAX_PAYLOAD_BYTES = 65536;
    static constexpr std::size_t MAX_REPUBLISHED = 32;

    static YonEvents& current() {
        static YonEvents events;
        return events;
    }

    void publish(const std::string& name, const json::JsonNode& value, bool retain) {
        std::string encoded = value.toJsonString();
        if (closed || queue.size() >= MAX_QUEUE || encoded.size() > MAX_PAYLOAD_BYTES) {
            report("Yon local event publication failed.");
            return;
        }
        if (retain) {
            retained[name] = encoded;
        }
        queue.push_back(YonPublished{name, encoded});
        send(name, encoded);
    }

    void receive(const std::string& name, const json::JsonNode& value) {
        queue.push_back(YonPublished{name, value.toJsonString()});
    }

    void only(std::vector<std::string> keys) {
        delivering = true;
        admitted = std::move(keys);
    }

    const json::JsonArray& published() const {
        return outbound;
    }

    void listen(const std::string& options, std::function<json::JsonNode(const json::JsonNode&)> callback) {
        json::JsonNode read = json::JsonReader::read(options);
        if (!admits(read["key"].getValue<std::string>())) {
            return;
        }
        for (const json::JsonNode& event : read["events"].items()) {
            if (listeners.size() >= MAX_LISTENERS) {
                throw std::runtime_error("Local listener capacity exceeded");
            }
            YonListenerEntry entry;
            entry.name = event.getValue<std::string>();
            entry.order = read["order"].getValue<long long>();
            entry.condition = read["condition"];
            entry.background = read["async"].getValue<bool>();
            entry.returns = read["returns"].isNull() ? std::string() : read["returns"].getValue<std::string>();
            entry.sequence = listeners.size();
            entry.callback = callback;
            replay(read["replay"].getValue<bool>(), entry.name);
            listeners.push_back(std::move(entry));
        }
        std::stable_sort(listeners.begin(), listeners.end(), [](const YonListenerEntry& left, const YonListenerEntry& right) {
            return left.order < right.order;
        });
    }

    void drain() {
        while (!queue.empty()) {
            YonPublished next = queue.front();
            queue.pop_front();
            deliveries += 1;
            if (deliveries > MAX_DELIVERIES) {
                queue.clear();
                report("Yon local event delivery bound exceeded.");
                return;
            }
            deliver(next);
        }
    }

    void close() {
        closed = true;
        queue.clear();
        listeners.clear();
    }

    static void report(const std::string& message) {
        std::string line = message + "\n";
        std::fwrite(line.data(), 1, line.size(), stderr);
    }

private:
    std::deque<YonPublished> queue;
    std::map<std::string, std::string> retained;
    std::vector<YonListenerEntry> listeners;
    std::size_t deliveries = 0;
    bool closed = false;
    bool delivering = false;
    std::vector<std::string> admitted;
    json::JsonArray outbound;

    bool admits(const std::string& key) const {
        return !delivering || std::find(admitted.begin(), admitted.end(), key) != admitted.end();
    }

    void send(const std::string& name, const std::string& encoded) {
        if (!YonTopics::named(name)) {
            return;
        }
        json::JsonObject topic;
        topic["topic"] = name;
        topic["payload_json"] = encoded;
        outbound.add(topic);
    }

    void replay(bool wanted, const std::string& name) {
        std::map<std::string, std::string>::const_iterator found = retained.find(name);
        if (wanted && found != retained.end()) {
            queue.push_back(YonPublished{name, found->second});
        }
    }

    void deliver(const YonPublished& published) {
        json::JsonNode value = json::JsonReader::read(published.encoded);
        std::vector<const YonListenerEntry*> deferred;
        for (const YonListenerEntry& listener : listeners) {
            if (!heard(listener, published.name, value)) {
                continue;
            }
            if (listener.background) {
                deferred.push_back(&listener);
                continue;
            }
            republish(run(listener, value), listener.returns);
        }
        for (const YonListenerEntry* listener : deferred) {
            run(*listener, value);
        }
    }

    static bool heard(const YonListenerEntry& listener, const std::string& name, const json::JsonNode& value) {
        bool named = listener.name == name;
        return named && (listener.condition.isNull() || YonCondition::test(listener.condition, value, name));
    }

    static json::JsonNode run(const YonListenerEntry& listener, const json::JsonNode& value) {
        try {
            return listener.callback(value);
        } catch (const json::JsonException&) {
            report("Yon local event payload does not fit the listener model.");
        } catch (...) {
            report("Yon local event listener failed.");
        }
        return json::JsonNode(nullptr);
    }

    void republish(const json::JsonNode& result, const std::string& returns) {
        if (returns.empty() || result.isNull()) {
            return;
        }
        if (result.getValueKind() != json::JsonValueKind::Array) {
            publish(returns, result, false);
            return;
        }
        std::size_t count = std::min(result.count(), MAX_REPUBLISHED);
        for (std::size_t index = 0; index < count; index += 1) {
            republishItem(result[index], returns);
        }
    }

    void republishItem(const json::JsonNode& item, const std::string& returns) {
        if (!item.isNull()) {
            publish(returns, item, false);
        }
    }
};

}

namespace tachyon {

class TacEventSink {
public:
    static void publish(const std::string& name, const json::JsonNode& value, bool retain) {
        yon::YonEvents::current().publish(name, value, retain);
    }
};

}
