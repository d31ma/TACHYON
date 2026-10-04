namespace tachyon {

class TacHost {
public:
    static std::string storage(const std::string&) {
        return missing();
    }

    static std::string web(const std::string&) {
        return missing();
    }

    static double epochMilliseconds() {
        return 0;
    }

    static void httpStart(std::uint32_t, const std::string&) {}

    static void httpCancel(std::uint32_t) {}

private:
    static std::string missing() {
        json::JsonObject failure;
        failure["error"] = "Browser APIs are not in C++ test units yet (ADR 0099).";
        return failure.toJsonString();
    }
};

}

namespace tachyon {

class TacTaskOwner {
public:
    static std::string current() {
        return std::string();
    }

    static void resumeFor(const std::string&, std::coroutine_handle<> waiting) {
        waiting.resume();
    }

    static void finish(std::uint32_t, const std::string&, const std::string&) {}
};

}
