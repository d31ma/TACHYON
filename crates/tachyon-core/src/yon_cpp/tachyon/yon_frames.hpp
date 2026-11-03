#ifdef _WIN32
#include <fcntl.h>
#include <io.h>
#endif

namespace tachyon::yon {

class YonFrames {
public:
    static constexpr std::size_t MAX_BYTES = 16 * 1024 * 1024;
    static constexpr std::size_t PREFIX_BYTES = 4;
    static constexpr int PROTOCOL_FAILURE = 70;

    static void useBinaryStreams() {
#ifdef _WIN32
        _setmode(_fileno(stdin), _O_BINARY);
        _setmode(_fileno(stdout), _O_BINARY);
#endif
    }

    static bool enabled() {
        const char* value = std::getenv("YON_RESOURCE_FRAMES");
        return value != nullptr && std::string(value) == "1";
    }

    static bool framed() {
        const char* value = std::getenv("YON_SOCKET_FRAMES");
        return value != nullptr && std::string(value) == "1";
    }

    static bool warm() {
        const char* value = std::getenv("YON_HANDLER_WARM");
        return value != nullptr && std::string(value) == "1";
    }

    static bool ended() {
        int next = std::fgetc(stdin);
        if (next == EOF) {
            return true;
        }
        std::ungetc(next, stdin);
        return false;
    }

    static bool read(std::string& payload) {
        unsigned char prefix[PREFIX_BYTES];
        if (std::fread(prefix, 1, PREFIX_BYTES, stdin) != PREFIX_BYTES) {
            return false;
        }
        std::size_t length = 0;
        for (unsigned char part : prefix) {
            length = (length << 8) | part;
        }
        if (length > MAX_BYTES) {
            return false;
        }
        payload.assign(length, '\0');
        return length == 0 || std::fread(payload.data(), 1, length, stdin) == length;
    }

    static std::string payload(const std::string& requestId, json::JsonNode fields) {
        fields["protocol_version"] = 1;
        fields["request_id"] = requestId;
        return YonText::repaired(fields.toJsonString());
    }

    static bool fits(const std::string& payload) {
        return payload.size() <= MAX_BYTES;
    }

    static bool write(const std::string& payload) {
        std::size_t length = payload.size();
        unsigned char prefix[PREFIX_BYTES] = {
            static_cast<unsigned char>(length >> 24),
            static_cast<unsigned char>(length >> 16),
            static_cast<unsigned char>(length >> 8),
            static_cast<unsigned char>(length),
        };
        bool written = std::fwrite(prefix, 1, PREFIX_BYTES, stdout) == PREFIX_BYTES
            && std::fwrite(payload.data(), 1, length, stdout) == length;
        return std::fflush(stdout) == 0 && written;
    }

    static bool send(const std::string& requestId, json::JsonNode fields) {
        std::string written = payload(requestId, std::move(fields));
        return fits(written) && write(written);
    }

    static json::JsonNode kind(const std::string& name) {
        json::JsonObject fields;
        fields["kind"] = name;
        return fields;
    }

    static json::JsonNode event(const std::string& data) {
        json::JsonObject body;
        body["encoding"] = "utf8";
        body["data"] = data;
        json::JsonNode fields = kind("event");
        fields["body"] = body;
        return fields;
    }

    static json::JsonNode failure(const std::string& code, const std::string& message) {
        json::JsonObject error;
        error["code"] = code;
        error["message"] = message;
        error["retryable"] = false;
        json::JsonNode fields = kind("response");
        fields["status"] = 500;
        fields["headers"] = json::JsonObject();
        fields["error"] = error;
        return fields;
    }

    static const json::JsonNode& field(const json::JsonNode& node, const std::string& key) {
        static const json::JsonNode nothing;
        if (node.getValueKind() != json::JsonValueKind::Object || !node.containsKey(key)) {
            return nothing;
        }
        return node[key];
    }

    static std::string text(const json::JsonNode& node, const std::string& key) {
        const json::JsonNode& value = field(node, key);
        if (value.getValueKind() != json::JsonValueKind::String) {
            return std::string();
        }
        return value.getValue<std::string>();
    }
};

}
