namespace tachyon {

class TacWebCall {
public:
    static json::JsonNode answer(const char* api, const char* call, json::JsonArray arguments = json::JsonArray()) {
        json::JsonObject request;
        request["api"] = api;
        request["call"] = call;
        request["args"] = std::move(arguments);
        json::JsonNode answered = json::JsonNode::parse(TacHost::web(request.toJsonString()));
        if (answered["error"].getValueKind() == json::JsonValueKind::String) {
            throw std::runtime_error(answered["error"].getValue<std::string>());
        }
        return answered["value"];
    }

    static std::string text(const char* api, const char* call) {
        return answer(api, call).getValue<std::string>();
    }

    static bool flag(const char* api, const char* call, json::JsonArray arguments = json::JsonArray()) {
        return answer(api, call, std::move(arguments)).getValue<bool>();
    }

    static double number(const char* api, const char* call) {
        return answer(api, call).getValue<double>();
    }

    template <typename... Values>
    static json::JsonArray arguments(Values... values) {
        json::JsonArray written;
        (written.add(json::JsonNode(values)), ...);
        return written;
    }
};

}

namespace tachyon::web {

class Clock {
public:
    double nowMs() const { return TacWebCall::number("clock", "now"); }

    double monotonicMs() const { return TacWebCall::number("clock", "monotonic"); }

    std::string timeZone() const { return TacWebCall::text("clock", "timeZone"); }
};

class SecureRandom {
public:
    std::vector<std::uint8_t> bytes(int count) const {
        json::JsonNode values = TacWebCall::answer("secureRandom", "bytes", TacWebCall::arguments(count));
        std::vector<std::uint8_t> bytes;
        for (const json::JsonNode& value : values.items()) {
            bytes.push_back(value.getValue<std::uint8_t>());
        }
        return bytes;
    }

    std::string uuid() const { return TacWebCall::text("secureRandom", "uuid"); }
};

class Connectivity {
public:
    bool online() const { return TacWebCall::flag("connectivity", "online"); }
};

class PageVisibility {
public:
    bool visible() const { return TacWebCall::flag("pageVisibility", "visible"); }
};

class Preferences {
public:
    std::string scheme() const { return TacWebCall::text("preferences", "scheme"); }

    std::string motion() const { return TacWebCall::text("preferences", "motion"); }

    std::string language() const { return TacWebCall::text("preferences", "language"); }
};

class Clipboard {
public:
    void writeText(const std::string& text) const { TacWebCall::answer("clipboard", "writeText", TacWebCall::arguments(text)); }

    void readText() const { TacWebCall::answer("clipboard", "readText"); }
};

class Share {
public:
    void open(const std::string& title, const std::string& text, const std::string& url) const {
        TacWebCall::answer("share", "open", TacWebCall::arguments(title, text, url));
    }
};

class Notifications {
public:
    std::string permission() const { return TacWebCall::text("notifications", "permission"); }

    void requestPermission() const { TacWebCall::answer("notifications", "requestPermission"); }

    void show(const std::string& title, const std::string& body) const {
        TacWebCall::answer("notifications", "show", TacWebCall::arguments(title, body));
    }
};

class Geolocation {
public:
    void currentPosition() const { TacWebCall::answer("geolocation", "currentPosition"); }
};

class WindowControls {
public:
    bool granted(const std::string& control) const { return TacWebCall::flag("windowControls", "granted", TacWebCall::arguments(control)); }

    void minimize() const { TacWebCall::answer("windowControls", "minimize"); }

    void maximize() const { TacWebCall::answer("windowControls", "maximize"); }

    void fullscreen() const { TacWebCall::answer("windowControls", "fullscreen"); }

    void close() const { TacWebCall::answer("windowControls", "close"); }

    void resize(int width, int height) const { TacWebCall::answer("windowControls", "resize", TacWebCall::arguments(width, height)); }

    void setTitle(const std::string& text) const { TacWebCall::answer("windowControls", "setTitle", TacWebCall::arguments(text)); }
};

}

using tachyon::web::Clipboard;
using tachyon::web::Clock;
using tachyon::web::Connectivity;
using tachyon::web::Geolocation;
using tachyon::web::Notifications;
using tachyon::web::PageVisibility;
using tachyon::web::Preferences;
using tachyon::web::SecureRandom;
using tachyon::web::Share;
using tachyon::web::WindowControls;
