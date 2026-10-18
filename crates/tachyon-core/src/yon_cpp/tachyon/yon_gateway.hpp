namespace tachyon::yon {

class YonFrames {
public:
    static constexpr std::size_t PREFIX_BYTES = 4;
    static constexpr std::size_t MAX_FRAME_BYTES = 16 * 1024 * 1024;

    static bool enabled() {
        const char* written = std::getenv("YON_RESOURCE_FRAMES");
        return written != nullptr && std::string(written) == "1";
    }

    static bool read(std::string& frame) {
        unsigned char prefix[PREFIX_BYTES];
        if (std::fread(prefix, 1, PREFIX_BYTES, stdin) != PREFIX_BYTES) {
            return false;
        }
        std::size_t length = (static_cast<std::size_t>(prefix[0]) << 24) | (static_cast<std::size_t>(prefix[1]) << 16) | (static_cast<std::size_t>(prefix[2]) << 8) | static_cast<std::size_t>(prefix[3]);
        if (length > MAX_FRAME_BYTES) {
            return false;
        }
        frame.assign(length, '\0');
        return length == 0 || std::fread(frame.data(), 1, length, stdin) == length;
    }

    static void write(const std::string& frame) {
        std::uint32_t length = static_cast<std::uint32_t>(frame.size());
        unsigned char prefix[PREFIX_BYTES] = {
            static_cast<unsigned char>(length >> 24),
            static_cast<unsigned char>(length >> 16),
            static_cast<unsigned char>(length >> 8),
            static_cast<unsigned char>(length),
        };
        std::fwrite(prefix, 1, PREFIX_BYTES, stdout);
        std::fwrite(frame.data(), 1, frame.size(), stdout);
        std::fflush(stdout);
    }
};

class YonGateway {
public:
    static constexpr const char* FAILED = "Gateway call failed.";

    static json::JsonNode call(const std::string& key, const std::vector<json::JsonNode>& arguments) {
        json::JsonNode answer = ask(key, arguments);
        bool settled = answer.getValueKind() == json::JsonValueKind::Object && answer.containsKey("result") && !answer.containsKey("error");
        if (!settled) {
            throw TacCodedError(FAILED, code(answer));
        }
        return answer["result"];
    }

private:
    static json::JsonNode ask(const std::string& key, const std::vector<json::JsonNode>& arguments) {
        if (!YonFrames::enabled()) {
            return failure("adapter");
        }
        json::JsonArray written;
        for (const json::JsonNode& argument : arguments) {
            written.add(argument);
        }
        json::JsonObject command;
        command["operation"] = "gateway";
        command["key"] = key;
        command["arguments"] = written;
        json::JsonObject asked;
        asked["kind"] = "resource";
        asked["command"] = command;
        YonFrames::write(asked.toJsonString());
        std::string raw;
        if (!YonFrames::read(raw)) {
            return failure("adapter");
        }
        try {
            return json::JsonNode::parse(raw);
        } catch (const std::exception&) {
            return failure("adapter");
        }
    }

    static json::JsonNode failure(const std::string& kind) {
        json::JsonObject failed;
        failed["error"] = kind;
        return failed;
    }

    static std::string code(const json::JsonNode& answer) {
        bool named = answer.getValueKind() == json::JsonValueKind::Object && answer.containsKey("error") && answer["error"].getValueKind() == json::JsonValueKind::String;
        std::string written = named ? answer["error"].getValue<std::string>() : std::string();
        bool known = written == "invalid" || written == "timeout" || written == "capacity";
        return known ? written : "adapter";
    }
};

}
