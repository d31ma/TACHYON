namespace tachyon::json {

class JsonWriter {
public:
    static std::string write(const JsonNode& node) {
        std::string output;
        append(output, node);
        return output;
    }

private:
    static constexpr const char* HEX_DIGITS = "0123456789abcdef";
    static constexpr int NUMBER_SPACE = 32;
    static constexpr int SHORT_PRECISION = 15;
    static constexpr int FULL_PRECISION = 17;

    static void append(std::string& output, const JsonNode& node) {
        switch (node.kind) {
        case JsonValueKind::True:
            output += "true";
            return;
        case JsonValueKind::False:
            output += "false";
            return;
        case JsonValueKind::Number:
            appendNumber(output, node.number);
            return;
        case JsonValueKind::String:
            appendText(output, node.text);
            return;
        case JsonValueKind::Array:
            appendArray(output, node);
            return;
        case JsonValueKind::Object:
            appendObject(output, node);
            return;
        default:
            output += "null";
            return;
        }
    }

    static void appendArray(std::string& output, const JsonNode& node) {
        output += '[';
        for (std::size_t index = 0; index < node.values.size(); index += 1) {
            if (index > 0) {
                output += ',';
            }
            append(output, node.values[index]);
        }
        output += ']';
    }

    static void appendObject(std::string& output, const JsonNode& node) {
        output += '{';
        for (std::size_t index = 0; index < node.keys.size(); index += 1) {
            if (index > 0) {
                output += ',';
            }
            appendText(output, node.keys[index]);
            output += ':';
            append(output, node.values[index]);
        }
        output += '}';
    }

    static void appendNumber(std::string& output, double value) {
        if (!std::isfinite(value)) {
            output += "null";
            return;
        }
        char digits[NUMBER_SPACE];
        for (int precision = SHORT_PRECISION; precision <= FULL_PRECISION; precision += 1) {
            std::snprintf(digits, NUMBER_SPACE, "%.*g", precision, value);
            if (std::strtod(digits, nullptr) == value) {
                break;
            }
        }
        output += digits;
    }

    static void appendText(std::string& output, const std::string& text) {
        output += '"';
        for (char character : text) {
            appendCharacter(output, character);
        }
        output += '"';
    }

    static void appendCharacter(std::string& output, char character) {
        switch (character) {
        case '"':
            output += "\\\"";
            return;
        case '\\':
            output += "\\\\";
            return;
        case '\n':
            output += "\\n";
            return;
        case '\r':
            output += "\\r";
            return;
        case '\t':
            output += "\\t";
            return;
        default:
            break;
        }
        unsigned char code = static_cast<unsigned char>(character);
        if (code >= 0x20) {
            output += character;
            return;
        }
        output += "\\u00";
        output += HEX_DIGITS[code >> 4];
        output += HEX_DIGITS[code & 0x0f];
    }
};

inline std::string JsonNode::toJsonString() const {
    return JsonWriter::write(*this);
}

}
