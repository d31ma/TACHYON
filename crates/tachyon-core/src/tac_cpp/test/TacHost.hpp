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

private:
    static std::string missing() {
        json::JsonObject failure;
        failure["error"] = "Browser APIs are not in C++ test units yet (ADR 0098).";
        return failure.toJsonString();
    }
};

}
