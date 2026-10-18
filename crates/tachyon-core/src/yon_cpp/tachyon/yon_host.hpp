#include <chrono>
#include <thread>

namespace tachyon {

class TacHost {
public:
    static constexpr const char* REALM = "yon";

    class Storage {
    public:
        virtual ~Storage() = default;
        virtual json::JsonNode answer(const json::JsonNode& request) = 0;
        virtual json::JsonNode partition() = 0;
    };

    static Storage*& current() {
        static Storage* running = nullptr;
        return running;
    }

    static Storage*& stores() {
        static Storage* kept = nullptr;
        return kept;
    }

    static std::string storage(const std::string& text) {
        json::JsonNode request = json::JsonNode::parse(text);
        Storage* running = stored(request) ? stores() : current();
        if (running == nullptr) {
            throw TacCodedError("This Yon C++ call has no behavior context. A class whose methods carry [[Cached]], [[UpdateCache]], [[ClearCache]] or [[AllOrNothing]] declares `BehaviorContext behaviors;` and opens it from a Setup (ADR 0099).", "invalid");
        }
        json::JsonObject answer;
        answer["value"] = running->answer(request);
        return answer.toJsonString();
    }

    static json::JsonNode partition(const json::JsonNode& written) {
        Storage* running = current();
        return running == nullptr ? written : running->partition();
    }

    static bool stored(const json::JsonNode& request) {
        bool entity = request["op"].getValueKind() == json::JsonValueKind::String && request["op"].getValue<std::string>() == "entity";
        return entity && request["command"]["entity"].getValueKind() == json::JsonValueKind::String;
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
