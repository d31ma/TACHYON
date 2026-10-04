namespace tachyon {

class AssertionFailed : public std::runtime_error {
public:
    explicit AssertionFailed(const std::string& message) : std::runtime_error(message) {}
};

class Assert {
public:
    template <typename Expected, typename Actual>
    static void equals(const Expected& expected, const Actual& actual, const std::string& message = "") {
        json::JsonNode wanted = node(expected);
        json::JsonNode found = node(actual);
        if (!json::JsonNode::deepEquals(wanted, found)) {
            fail(message, "expected: <" + shown(wanted) + "> but was: <" + shown(found) + ">");
        }
    }

    static void isTrue(bool condition, const std::string& message = "") {
        if (!condition) {
            fail(message, "expected: <true> but was: <false>");
        }
    }

    static void isFalse(bool condition, const std::string& message = "") {
        if (condition) {
            fail(message, "expected: <false> but was: <true>");
        }
    }

    template <typename Action>
    static std::string throws(Action action, const std::string& message = "") {
        try {
            action();
        } catch (const AssertionFailed&) {
            throw;
        } catch (const std::exception& problem) {
            return problem.what();
        }
        fail(message, "expected an exception but none was thrown");
    }

private:
    template <typename Value>
    static json::JsonNode node(const Value& value) {
        if constexpr (std::is_array_v<Value>) {
            return json::JsonNode(std::string(value));
        } else if constexpr (std::is_same_v<Value, const char*>) {
            return json::JsonNode(std::string(value));
        } else {
            return json::JsonSerializer::serializeToNode(value);
        }
    }

    static std::string shown(const json::JsonNode& value) {
        if (value.getValueKind() == json::JsonValueKind::String) {
            return value.getValue<std::string>();
        }
        return value.toJsonString();
    }

    [[noreturn]] static void fail(const std::string& message, const std::string& detail) {
        std::string prefix = message.empty() ? std::string() : message + " ==> ";
        throw AssertionFailed(prefix + detail);
    }
};

}
