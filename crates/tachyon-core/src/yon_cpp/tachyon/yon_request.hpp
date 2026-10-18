namespace tachyon::yon {

class YonRequest {
public:
    static http::HttpRequestMessage of(const YonFrame& frame) {
        std::string target = frame.text("target");
        http::HttpRequestMessage request(http::HttpMethod(frame.text("method")), target.empty() ? frame.text("route") : target);
        for (const auto& [name, values] : listed(frame.field("headers"))) {
            request.headers.add(name, joined(values));
        }
        std::map<std::string, std::string> body = listed<std::string>(frame.field("body"));
        if (body.count("data") > 0) {
            request.content = http::HttpContent(body["encoding"] == "base64" ? decoded(body["data"]) : body["data"]);
        }
        return request;
    }

private:
    template <typename T = std::vector<std::string>>
    static std::map<std::string, T> listed(const json::JsonNode& node) {
        if (node.getValueKind() != json::JsonValueKind::Object) {
            return {};
        }
        return json::JsonSerializer::deserialize<std::map<std::string, T>>(node);
    }

    static std::string joined(const std::vector<std::string>& values) {
        std::string text;
        for (const std::string& value : values) {
            text += (text.empty() ? "" : ", ") + value;
        }
        return text;
    }

    static std::string decoded(const std::string& written) {
        static const std::string ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        std::string bytes;
        unsigned collected = 0;
        int bits = 0;
        for (char character : written) {
            std::size_t value = ALPHABET.find(character);
            if (value == std::string::npos) {
                continue;
            }
            collected = (collected << 6) | static_cast<unsigned>(value);
            bits += 6;
            if (bits >= 8) {
                bits -= 8;
                bytes.push_back(static_cast<char>((collected >> bits) & 0xFF));
            }
        }
        return bytes;
    }
};

}
