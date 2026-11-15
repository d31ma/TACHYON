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

class Focus {
public:
    void move(const std::string& target) const { TacWebCall::answer("focus", "move", TacWebCall::arguments(target)); }

    void trap(const std::string& container) const { TacWebCall::answer("focus", "trap", TacWebCall::arguments(container)); }

    void release() const { TacWebCall::answer("focus", "release"); }
};

class Files {
public:
    void pick(const std::string& accept, bool multiple) const {
        TacWebCall::answer("files", "pick", TacWebCall::arguments(accept, multiple));
    }

    void read(const std::string& id, long long offset, int count) const {
        TacWebCall::answer("files", "read", TacWebCall::arguments(id, offset, count));
    }

    void release(const std::string& id) const { TacWebCall::answer("files", "release", TacWebCall::arguments(id)); }

    std::string upload(const std::string& id, const std::string& method, const std::string& url, const std::string& headers) const {
        return TacWebCall::answer("files", "upload", TacWebCall::arguments(id, method, url, headers)).getValue<std::string>();
    }

    void cancel(const std::string& upload) const { TacWebCall::answer("files", "cancel", TacWebCall::arguments(upload)); }
};

class Notifications {
public:
    std::string permission() const { return TacWebCall::text("notifications", "permission"); }

    void requestPermission() const { TacWebCall::answer("notifications", "requestPermission"); }

    void show(const std::string& title, const std::string& body) const {
        TacWebCall::answer("notifications", "show", TacWebCall::arguments(title, body));
    }
};

class Push {
public:
    bool supported() const { return TacWebCall::flag("push", "supported"); }

    void subscribe(const std::string& applicationServerKey) const {
        TacWebCall::answer("push", "subscribe", TacWebCall::arguments(applicationServerKey));
    }

    void subscription() const { TacWebCall::answer("push", "subscription"); }

    void unsubscribe() const { TacWebCall::answer("push", "unsubscribe"); }
};

class StorageSpace {
public:
    void estimate() const { TacWebCall::answer("storageSpace", "estimate"); }

    void persist() const { TacWebCall::answer("storageSpace", "persist"); }
};

class Passkeys {
public:
    bool supported() const { return TacWebCall::flag("passkeys", "supported"); }

    void check() const { TacWebCall::answer("passkeys", "check"); }

    void create(const std::string& optionsJson, const std::string& prfKey) const {
        TacWebCall::answer("passkeys", "create", TacWebCall::arguments(optionsJson, prfKey));
    }

    void get(const std::string& optionsJson, const std::string& prfKey) const {
        TacWebCall::answer("passkeys", "get", TacWebCall::arguments(optionsJson, prfKey));
    }

    void offer(const std::string& optionsJson, const std::string& prfKey) const {
        TacWebCall::answer("passkeys", "offer", TacWebCall::arguments(optionsJson, prfKey));
    }

    void cancel() const { TacWebCall::answer("passkeys", "cancel"); }
};

class Media {
public:
    bool supported(const std::string& feature) const { return TacWebCall::flag("media", "supported", TacWebCall::arguments(feature)); }

    void devices(const std::string& request) const { TacWebCall::answer("media", "devices", TacWebCall::arguments(request)); }

    void chooseSpeaker(const std::string& device) const {
        TacWebCall::answer("media", "chooseSpeaker", TacWebCall::arguments(device));
    }

    void captureMicrophone(const std::string& track, const std::string& device, bool processing) const {
        TacWebCall::answer("media", "captureMicrophone", TacWebCall::arguments(track, device, processing));
    }

    void captureCamera(const std::string& track, const std::string& device, const std::string& quality) const {
        TacWebCall::answer("media", "captureCamera", TacWebCall::arguments(track, device, quality));
    }

    void captureScreen(const std::string& track, bool withAudio) const {
        TacWebCall::answer("media", "captureScreen", TacWebCall::arguments(track, withAudio));
    }

    void stopTrack(const std::string& track) const { TacWebCall::answer("media", "stopTrack", TacWebCall::arguments(track)); }

    void mute(const std::string& track, bool muted) const { TacWebCall::answer("media", "mute", TacWebCall::arguments(track, muted)); }

    void useFrameKey(long long keyId, const std::string& key) const {
        TacWebCall::answer("media", "useFrameKey", TacWebCall::arguments(keyId, key));
    }

    void sendFrameKey(long long keyId) const { TacWebCall::answer("media", "sendFrameKey", TacWebCall::arguments(keyId)); }

    void forgetFrameKey(long long keyId) const { TacWebCall::answer("media", "forgetFrameKey", TacWebCall::arguments(keyId)); }

    void encryptFrames(const std::string& connection, bool on) const {
        TacWebCall::answer("media", "encryptFrames", TacWebCall::arguments(connection, on));
    }
};

class CodeScanner {
public:
    bool supported() const { return TacWebCall::flag("codeScanner", "supported"); }

    void scan() const { TacWebCall::answer("codeScanner", "scan"); }

    void cancel() const { TacWebCall::answer("codeScanner", "cancel"); }
};

class Crypto {
public:
    bool supported() const { return TacWebCall::flag("crypto", "supported"); }

    std::string check() const { return request("check", TacWebCall::arguments()); }

    std::string makeKeyPair(const std::string& key, const std::string& algorithm, bool keep) const {
        return request("makeKeyPair", TacWebCall::arguments(key, algorithm, keep));
    }

    std::string publicKey(const std::string& key) const { return request("publicKey", TacWebCall::arguments(key)); }

    std::string sign(const std::string& key, const std::string& data) const {
        return request("sign", TacWebCall::arguments(key, data));
    }

    std::string verify(const std::string& publicKey, const std::string& data, const std::string& signature) const {
        return request("verify", TacWebCall::arguments(publicKey, data, signature));
    }

    std::string agree(const std::string& key, const std::string& peerPublicKey, const std::string& salt, const std::string& info, const std::string& newKey, const std::string& use, bool keep) const {
        return request("agree", TacWebCall::arguments(key, peerPublicKey, salt, info, newKey, use, keep));
    }

    std::string deriveKey(const std::string& fromKey, const std::string& salt, const std::string& info, const std::string& newKey, const std::string& use, bool keep) const {
        return request("deriveKey", TacWebCall::arguments(fromKey, salt, info, newKey, use, keep));
    }

    std::string importKey(const std::string& newKey, const std::string& secret, const std::string& use, bool keep) const {
        return request("importKey", TacWebCall::arguments(newKey, secret, use, keep));
    }

    std::string fromCode(const std::string& newKey, const std::string& code, const std::string& salt, const std::string& use, bool keep) const {
        return request("fromCode", TacWebCall::arguments(newKey, code, salt, use, keep));
    }

    std::string encrypt(const std::string& key, const std::string& data, const std::string& aad) const {
        return request("encrypt", TacWebCall::arguments(key, data, aad));
    }

    std::string decrypt(const std::string& key, const std::string& sealed, const std::string& aad) const {
        return request("decrypt", TacWebCall::arguments(key, sealed, aad));
    }

    std::string hkdf(const std::string& input, const std::string& salt, const std::string& info, int length) const {
        return request("hkdf", TacWebCall::arguments(input, salt, info, length));
    }

    std::string hash(const std::string& data) const { return request("hash", TacWebCall::arguments(data)); }

    std::string forget(const std::string& key) const { return request("forget", TacWebCall::arguments(key)); }

    std::string forgetAll() const { return request("forgetAll", TacWebCall::arguments()); }

private:
    static std::string request(const char* call, json::JsonArray arguments) {
        return TacWebCall::answer("crypto", call, arguments).getValue<std::string>();
    }
};

class Blobs {
public:
    void saveHeld(const std::string& fileId, const std::string& key) const {
        TacWebCall::answer("blobs", "saveHeld", TacWebCall::arguments(fileId, key));
    }

    void download(const std::string& url, const std::string& key, const std::string& headers) const {
        TacWebCall::answer("blobs", "download", TacWebCall::arguments(url, key, headers));
    }

    void list(const std::string& prefix) const { TacWebCall::answer("blobs", "list", TacWebCall::arguments(prefix)); }

    void read(const std::string& key, long long offset, int count) const {
        TacWebCall::answer("blobs", "read", TacWebCall::arguments(key, offset, count));
    }

    void remove(const std::string& key) const { TacWebCall::answer("blobs", "remove", TacWebCall::arguments(key)); }

    std::string url(const std::string& key) const {
        return TacWebCall::answer("blobs", "url", TacWebCall::arguments(key)).getValue<std::string>();
    }

    void open(const std::string& key) const { TacWebCall::answer("blobs", "open", TacWebCall::arguments(key)); }

    void close(const std::string& key) const { TacWebCall::answer("blobs", "close", TacWebCall::arguments(key)); }

    std::string upload(const std::string& key, const std::string& method, const std::string& url, const std::string& headers) const {
        return TacWebCall::answer("blobs", "upload", TacWebCall::arguments(key, method, url, headers)).getValue<std::string>();
    }

    void cancel(const std::string& upload) const { TacWebCall::answer("blobs", "cancel", TacWebCall::arguments(upload)); }
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
using tachyon::web::Focus;
using tachyon::web::Geolocation;
using tachyon::web::Notifications;
using tachyon::web::PageVisibility;
using tachyon::web::Push;
using tachyon::web::Preferences;
using tachyon::web::SecureRandom;
using tachyon::web::Share;
using tachyon::web::StorageSpace;
using tachyon::web::Passkeys;
using tachyon::web::Blobs;
using tachyon::web::Media;
using tachyon::web::CodeScanner;
using tachyon::web::Crypto;
using tachyon::web::Files;
using tachyon::web::WindowControls;
