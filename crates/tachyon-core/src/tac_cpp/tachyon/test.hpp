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

namespace tachyon {

class BehaviorCalls {
public:
    static void record(const char* key) {
        std::vector<std::string>& keys = recorded();
        if (std::find(keys.begin(), keys.end(), key) == keys.end()) {
            keys.emplace_back(key);
        }
    }

    static json::JsonArray take() {
        json::JsonArray called;
        for (const std::string& key : recorded()) {
            called.add(key);
        }
        recorded().clear();
        return called;
    }

private:
    static std::vector<std::string>& recorded() {
        static std::vector<std::string> keys;
        return keys;
    }
};

class TacTestCase {
public:
    template <typename Call>
    static void finish(Call call) {
        using Answer = decltype(call());
        if constexpr (TacIsTask<Answer>::value) {
            Answer task = call();
            if (!task.await_ready()) {
                throw std::runtime_error("The test case waits for something that never answers in a C++ test unit.");
            }
            task.await_resume();
        } else {
            call();
        }
    }
};

class TacTestRunner {
public:
    explicit TacTestRunner(std::string unit) {
        report["unit"] = unit;
        report["cases"] = json::JsonArray();
    }

    template <typename Case>
    void run(const std::string& name, Case testCase) {
        json::JsonObject result;
        result["name"] = name;
        result["passed"] = true;
        BehaviorCalls::take();
        try {
            testCase();
        } catch (const std::exception& problem) {
            result["passed"] = false;
            result["error"] = std::string(problem.what());
        } catch (...) {
            result["passed"] = false;
            result["error"] = "The case threw something that is not a std::exception.";
        }
        result["called"] = BehaviorCalls::take();
        report["cases"].add(result);
    }

    std::string text() const {
        return report.toJsonString();
    }

private:
    json::JsonObject report;
};

}

using tachyon::Assert;
using tachyon::BehaviorCalls;
