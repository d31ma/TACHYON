#include <chrono>
#include <thread>

namespace tachyon {

class TacHost {
public:
    static constexpr const char* REALM = "yon";

    static std::string storage(const std::string&) {
        json::JsonObject failure;
        failure["error"] = "Yon C++ code has no behavior storage yet (ADR 0099).";
        return failure.toJsonString();
    }

    static double epochMilliseconds() {
        auto since = std::chrono::system_clock::now().time_since_epoch();
        return static_cast<double>(std::chrono::duration_cast<std::chrono::milliseconds>(since).count());
    }

    static void waitMilliseconds(double milliseconds) {
        std::this_thread::sleep_for(std::chrono::duration<double, std::milli>(milliseconds));
    }
};

}
