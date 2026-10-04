namespace tachyon {

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
        try {
            testCase();
        } catch (const std::exception& problem) {
            result["passed"] = false;
            result["error"] = std::string(problem.what());
        } catch (...) {
            result["passed"] = false;
            result["error"] = "The case threw something that is not a std::exception.";
        }
        report["cases"].add(result);
    }

    std::string text() const {
        return report.toJsonString();
    }

private:
    json::JsonObject report;
};

}
