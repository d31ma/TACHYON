namespace tachyon::json {

class JsonReader {
public:
    static constexpr std::size_t MAX_BYTES = 1048576;
    static constexpr int MAX_DEPTH = 64;

    static JsonNode read(std::string_view json) {
        if (json.size() > MAX_BYTES) {
            throw JsonException("The JSON text is larger than 1 MiB.");
        }
        JsonReader reader(json);
        reader.skipSpace();
        JsonNode node = reader.readValue(0);
        reader.skipSpace();
        if (reader.position != json.size()) {
            reader.fail("unexpected text after the value");
        }
        return node;
    }

private:
    static constexpr unsigned SURROGATE_FIRST = 0xD800;
    static constexpr unsigned SURROGATE_SECOND = 0xDC00;
    static constexpr unsigned SURROGATE_END = 0xE000;

    std::string_view json;
    std::size_t position = 0;

    explicit JsonReader(std::string_view json) : json(json) {}

    [[noreturn]] void fail(const std::string& problem) const {
        throw JsonException("Invalid JSON at byte " + std::to_string(position) + ": " + problem + ".");
    }

    bool atEnd() const { return position >= json.size(); }

    char current() const { return atEnd() ? '\0' : json[position]; }

    void skipSpace() {
        while (!atEnd() && (current() == ' ' || current() == '\n' || current() == '\r' || current() == '\t')) {
            position += 1;
        }
    }

    void expectWord(std::string_view word) {
        if (json.substr(position, word.size()) != word) {
            fail("unknown word");
        }
        position += word.size();
    }

    JsonNode readValue(int depth) {
        if (depth > MAX_DEPTH) {
            fail("the value is nested more than 64 levels deep");
        }
        char start = current();
        if (start == '{') {
            return readObject(depth);
        }
        if (start == '[') {
            return readArray(depth);
        }
        if (start == '"') {
            return JsonNode(readText());
        }
        if (start == 't') {
            expectWord("true");
            return JsonNode(true);
        }
        if (start == 'f') {
            expectWord("false");
            return JsonNode(false);
        }
        if (start == 'n') {
            expectWord("null");
            return JsonNode();
        }
        return JsonNode(readNumber());
    }

    JsonNode readObject(int depth) {
        JsonObject object;
        position += 1;
        skipSpace();
        if (current() == '}') {
            position += 1;
            return object;
        }
        while (true) {
            skipSpace();
            if (current() != '"') {
                fail("an object key must be a string");
            }
            std::string key = readText();
            skipSpace();
            if (current() != ':') {
                fail("a colon must follow an object key");
            }
            position += 1;
            skipSpace();
            object[key] = readValue(depth + 1);
            skipSpace();
            if (current() == '}') {
                position += 1;
                return object;
            }
            if (current() != ',') {
                fail("a comma or a closing brace must follow an object member");
            }
            position += 1;
        }
    }

    JsonNode readArray(int depth) {
        JsonArray array;
        position += 1;
        skipSpace();
        if (current() == ']') {
            position += 1;
            return array;
        }
        while (true) {
            skipSpace();
            array.add(readValue(depth + 1));
            skipSpace();
            if (current() == ']') {
                position += 1;
                return array;
            }
            if (current() != ',') {
                fail("a comma or a closing bracket must follow an array item");
            }
            position += 1;
        }
    }

    std::size_t digitsFrom(std::size_t start) const {
        std::size_t end = start;
        while (end < json.size() && json[end] >= '0' && json[end] <= '9') {
            end += 1;
        }
        return end - start;
    }

    double readNumber() {
        std::size_t start = position;
        if (current() == '-') {
            position += 1;
        }
        std::size_t whole = digitsFrom(position);
        if (whole == 0 || (whole > 1 && current() == '0')) {
            fail("a number has a wrong form");
        }
        position += whole;
        if (current() == '.') {
            position += 1;
            std::size_t fraction = digitsFrom(position);
            if (fraction == 0) {
                fail("a number has no digits after the decimal point");
            }
            position += fraction;
        }
        if (current() == 'e' || current() == 'E') {
            position += 1;
            if (current() == '+' || current() == '-') {
                position += 1;
            }
            std::size_t exponent = digitsFrom(position);
            if (exponent == 0) {
                fail("a number has no digits in its exponent");
            }
            position += exponent;
        }
        std::string written(json.substr(start, position - start));
        return std::strtod(written.c_str(), nullptr);
    }

    unsigned readHex() {
        if (position + 4 > json.size()) {
            fail("a unicode escape needs four hex digits");
        }
        unsigned value = 0;
        for (int index = 0; index < 4; index += 1) {
            char digit = json[position];
            position += 1;
            value = value * 16 + hexValue(digit);
        }
        return value;
    }

    unsigned hexValue(char digit) const {
        if (digit >= '0' && digit <= '9') {
            return static_cast<unsigned>(digit - '0');
        }
        if (digit >= 'a' && digit <= 'f') {
            return static_cast<unsigned>(digit - 'a' + 10);
        }
        if (digit >= 'A' && digit <= 'F') {
            return static_cast<unsigned>(digit - 'A' + 10);
        }
        fail("a unicode escape has a character that is not a hex digit");
    }

    unsigned readCodePoint() {
        unsigned first = readHex();
        if (first < SURROGATE_FIRST || first >= SURROGATE_END) {
            return first;
        }
        if (first >= SURROGATE_SECOND || json.substr(position, 2) != "\\u") {
            fail("a unicode escape has a lone surrogate");
        }
        position += 2;
        unsigned second = readHex();
        if (second < SURROGATE_SECOND || second >= SURROGATE_END) {
            fail("a unicode escape has a lone surrogate");
        }
        return 0x10000 + ((first - SURROGATE_FIRST) << 10) + (second - SURROGATE_SECOND);
    }

    static void appendUtf8(std::string& text, unsigned code) {
        if (code < 0x80) {
            text += static_cast<char>(code);
            return;
        }
        if (code < 0x800) {
            text += static_cast<char>(0xC0 | (code >> 6));
            text += static_cast<char>(0x80 | (code & 0x3F));
            return;
        }
        if (code < 0x10000) {
            text += static_cast<char>(0xE0 | (code >> 12));
            text += static_cast<char>(0x80 | ((code >> 6) & 0x3F));
            text += static_cast<char>(0x80 | (code & 0x3F));
            return;
        }
        text += static_cast<char>(0xF0 | (code >> 18));
        text += static_cast<char>(0x80 | ((code >> 12) & 0x3F));
        text += static_cast<char>(0x80 | ((code >> 6) & 0x3F));
        text += static_cast<char>(0x80 | (code & 0x3F));
    }

    void readEscape(std::string& text) {
        char escaped = current();
        position += 1;
        switch (escaped) {
        case '"':
        case '\\':
        case '/':
            text += escaped;
            return;
        case 'b':
            text += '\b';
            return;
        case 'f':
            text += '\f';
            return;
        case 'n':
            text += '\n';
            return;
        case 'r':
            text += '\r';
            return;
        case 't':
            text += '\t';
            return;
        case 'u':
            appendUtf8(text, readCodePoint());
            return;
        default:
            fail("a string has an unknown escape");
        }
    }

    std::string readText() {
        std::string text;
        position += 1;
        while (true) {
            if (atEnd()) {
                fail("a string has no closing quote");
            }
            char character = current();
            position += 1;
            if (character == '"') {
                return text;
            }
            if (static_cast<unsigned char>(character) < 0x20) {
                fail("a string has a control character");
            }
            if (character == '\\') {
                readEscape(text);
                continue;
            }
            text += character;
        }
    }
};

inline JsonNode JsonNode::parse(std::string_view json) {
    return JsonReader::read(json);
}

}
