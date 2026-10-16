namespace tachyon::json {

enum class JsonValueKind { Undefined, Object, Array, String, Number, True, False, Null };

}

namespace tachyon::json {

class JsonException : public std::runtime_error {
public:
    explicit JsonException(const std::string& message) : std::runtime_error(message) {}
};

}

namespace tachyon::json {

class JsonNode {
public:
    JsonNode() = default;
    JsonNode(std::nullptr_t) {}
    JsonNode(bool value) : kind(value ? JsonValueKind::True : JsonValueKind::False) {}
    JsonNode(int value) : kind(JsonValueKind::Number), number(value) {}
    JsonNode(long value) : kind(JsonValueKind::Number), number(static_cast<double>(value)) {}
    JsonNode(long long value) : kind(JsonValueKind::Number), number(static_cast<double>(value)) {}
    JsonNode(unsigned value) : kind(JsonValueKind::Number), number(value) {}
    JsonNode(unsigned long value) : kind(JsonValueKind::Number), number(static_cast<double>(value)) {}
    JsonNode(unsigned long long value) : kind(JsonValueKind::Number), number(static_cast<double>(value)) {}
    JsonNode(float value) : kind(JsonValueKind::Number), number(value) {}
    JsonNode(double value) : kind(JsonValueKind::Number), number(value) {}
    JsonNode(const char* value) : kind(JsonValueKind::String), text(value) {}
    JsonNode(std::string value) : kind(JsonValueKind::String), text(std::move(value)) {}
    JsonNode(std::string_view value) : kind(JsonValueKind::String), text(value) {}

    static JsonNode parse(std::string_view json);

    std::string toJsonString() const;

    JsonValueKind getValueKind() const { return kind; }

    bool isNull() const { return kind == JsonValueKind::Null; }

    template <typename T>
    T getValue() const {
        if constexpr (std::is_same_v<T, bool>) {
            return booleanValue();
        } else if constexpr (std::is_integral_v<T>) {
            return integerValue<T>();
        } else if constexpr (std::is_floating_point_v<T>) {
            return static_cast<T>(numberValue());
        } else {
            static_assert(std::is_same_v<T, std::string>, "getValue reads bool, numbers and std::string. Use JsonSerializer::deserialize for other types.");
            requireKind(JsonValueKind::String, "a string");
            return text;
        }
    }

    bool containsKey(const std::string& key) const { return keyIndex(key) < keys.size(); }

    JsonNode& operator[](const std::string& key) {
        requireKind(JsonValueKind::Object, "an object");
        std::size_t index = keyIndex(key);
        if (index < keys.size()) {
            return values[index];
        }
        keys.push_back(key);
        values.emplace_back();
        return values.back();
    }

    const JsonNode& operator[](const std::string& key) const {
        requireKind(JsonValueKind::Object, "an object");
        std::size_t index = keyIndex(key);
        if (index < keys.size()) {
            return values[index];
        }
        return missing();
    }

    JsonNode& operator[](std::size_t index) {
        requireKind(JsonValueKind::Array, "an array");
        requireIndex(index);
        return values[index];
    }

    const JsonNode& operator[](std::size_t index) const {
        requireKind(JsonValueKind::Array, "an array");
        requireIndex(index);
        return values[index];
    }

    std::size_t count() const { return values.size(); }

    void add(JsonNode item) {
        requireKind(JsonValueKind::Array, "an array");
        values.push_back(std::move(item));
    }

    bool remove(const std::string& key) {
        requireKind(JsonValueKind::Object, "an object");
        std::size_t index = keyIndex(key);
        if (index >= keys.size()) {
            return false;
        }
        keys.erase(keys.begin() + static_cast<std::ptrdiff_t>(index));
        values.erase(values.begin() + static_cast<std::ptrdiff_t>(index));
        return true;
    }

    const std::vector<std::string>& propertyNames() const { return keys; }

    const std::vector<JsonNode>& items() const { return values; }

    JsonNode deepClone() const { return *this; }

    static bool deepEquals(const JsonNode& left, const JsonNode& right) {
        return left.kind == right.kind && left.number == right.number && left.text == right.text
            && left.keys == right.keys && sameValues(left.values, right.values);
    }

    bool operator==(const JsonNode& other) const { return deepEquals(*this, other); }

protected:
    explicit JsonNode(JsonValueKind kind) : kind(kind) {}

private:
    JsonValueKind kind = JsonValueKind::Null;
    double number = 0;
    std::string text;
    std::vector<std::string> keys;
    std::vector<JsonNode> values;

    friend class JsonWriter;

    static const JsonNode& missing() {
        static const JsonNode nothing;
        return nothing;
    }

    static bool sameValues(const std::vector<JsonNode>& left, const std::vector<JsonNode>& right) {
        if (left.size() != right.size()) {
            return false;
        }
        for (std::size_t index = 0; index < left.size(); index += 1) {
            if (!deepEquals(left[index], right[index])) {
                return false;
            }
        }
        return true;
    }

    std::size_t keyIndex(const std::string& key) const {
        for (std::size_t index = 0; index < keys.size(); index += 1) {
            if (keys[index] == key) {
                return index;
            }
        }
        return keys.size();
    }

    void requireKind(JsonValueKind wanted, const char* word) const {
        if (kind != wanted) {
            throw JsonException(std::string("The JSON value is not ") + word + ".");
        }
    }

    void requireIndex(std::size_t index) const {
        if (index >= values.size()) {
            throw JsonException("The JSON array has no item " + std::to_string(index) + ".");
        }
    }

    double numberValue() const {
        requireKind(JsonValueKind::Number, "a number");
        return number;
    }

    bool booleanValue() const {
        if (kind == JsonValueKind::True) {
            return true;
        }
        requireKind(JsonValueKind::False, "a boolean");
        return false;
    }

    template <typename T>
    T integerValue() const {
        double value = numberValue();
        double lowest = static_cast<double>(std::numeric_limits<T>::lowest());
        double highest = static_cast<double>(std::numeric_limits<T>::max());
        bool exact = std::numeric_limits<T>::digits < std::numeric_limits<double>::digits;
        bool belowTop = exact ? value <= highest : value < highest;
        if (std::floor(value) != value || value < lowest || !belowTop) {
            throw JsonException("The JSON number does not fit the integer type.");
        }
        return static_cast<T>(value);
    }

};

}

namespace tachyon::json {

class JsonObject : public JsonNode {
public:
    JsonObject() : JsonNode(JsonValueKind::Object) {}
};

}

namespace tachyon::json {

class JsonArray : public JsonNode {
public:
    JsonArray() : JsonNode(JsonValueKind::Array) {}
};

}

namespace tachyon::json {

class JsonValue : public JsonNode {
public:
    template <typename T>
    static JsonNode create(T value) {
        return JsonNode(value);
    }
};

}

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

namespace tachyon::json {

class JsonReader {
public:
    static constexpr std::size_t MAX_BYTES = 1048576;
    static constexpr int MAX_DEPTH = 64;

    static JsonNode read(std::string_view json, std::size_t limit = MAX_BYTES) {
        if (json.size() > limit) {
            throw JsonException("The JSON text is larger than " + std::to_string(limit / MAX_BYTES) + " MiB.");
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

namespace tachyon::json {

template <typename T>
struct JsonUnknownType : std::false_type {};

template <typename T, typename Enable = void>
struct JsonCodec {
    static_assert(JsonUnknownType<T>::value, "Tachyon has no JSON codec for this type. Use bool, a number, std::string, std::vector, std::optional, std::map with std::string keys, JsonNode or a model from client/models.");
};

template <>
struct JsonCodec<bool> {
    static JsonNode toJson(bool value) { return JsonNode(value); }
    static bool fromJson(const JsonNode& node) { return node.getValue<bool>(); }
};

template <typename T>
struct JsonCodec<T, std::enable_if_t<std::is_arithmetic_v<T> && !std::is_same_v<T, bool>>> {
    static JsonNode toJson(T value) { return JsonNode(value); }
    static T fromJson(const JsonNode& node) { return node.getValue<T>(); }
};

template <>
struct JsonCodec<std::string> {
    static JsonNode toJson(const std::string& value) { return JsonNode(value); }
    static std::string fromJson(const JsonNode& node) { return node.getValue<std::string>(); }
};

template <>
struct JsonCodec<JsonNode> {
    static JsonNode toJson(const JsonNode& value) { return value; }
    static JsonNode fromJson(const JsonNode& node) { return node; }
};

template <typename T>
struct JsonCodec<std::vector<T>> {
    static JsonNode toJson(const std::vector<T>& value) {
        JsonArray array;
        for (const T& item : value) {
            array.add(JsonCodec<T>::toJson(item));
        }
        return array;
    }

    static std::vector<T> fromJson(const JsonNode& node) {
        if (node.getValueKind() != JsonValueKind::Array) {
            throw JsonException("The JSON value is not an array.");
        }
        std::vector<T> value;
        for (const JsonNode& item : node.items()) {
            value.push_back(JsonCodec<T>::fromJson(item));
        }
        return value;
    }
};

template <typename T>
struct JsonCodec<std::optional<T>> {
    static JsonNode toJson(const std::optional<T>& value) {
        if (!value.has_value()) {
            return JsonNode();
        }
        return JsonCodec<T>::toJson(*value);
    }

    static std::optional<T> fromJson(const JsonNode& node) {
        if (node.isNull()) {
            return std::nullopt;
        }
        return JsonCodec<T>::fromJson(node);
    }
};

template <typename T>
struct JsonCodec<std::map<std::string, T>> {
    static JsonNode toJson(const std::map<std::string, T>& value) {
        JsonObject object;
        for (const auto& [key, item] : value) {
            object[key] = JsonCodec<T>::toJson(item);
        }
        return object;
    }

    static std::map<std::string, T> fromJson(const JsonNode& node) {
        if (node.getValueKind() != JsonValueKind::Object) {
            throw JsonException("The JSON value is not an object.");
        }
        std::map<std::string, T> value;
        for (const std::string& key : node.propertyNames()) {
            value[key] = JsonCodec<T>::fromJson(node[key]);
        }
        return value;
    }
};

}

namespace tachyon::json {

class JsonModelFields {
public:
    template <typename T>
    static void write(JsonNode& object, const char* key, const T& value) {
        object[key] = JsonCodec<T>::toJson(value);
    }

    template <typename T>
    static void read(const JsonNode& object, const char* key, T& field) {
        if (object.containsKey(key)) {
            field = JsonCodec<T>::fromJson(object[key]);
        }
    }

    static void requireObject(const JsonNode& node, std::string_view model) {
        if (node.getValueKind() != JsonValueKind::Object) {
            throw JsonException("The JSON value for " + std::string(model) + " is not an object.");
        }
    }

    static std::string enumName(const JsonNode& node, std::string_view model) {
        if (node.getValueKind() != JsonValueKind::String) {
            throw JsonException("The JSON value for " + std::string(model) + " is not a name.");
        }
        return node.getValue<std::string>();
    }

    [[noreturn]] static void unknownName(std::string_view model) {
        throw JsonException("The JSON text is not a " + std::string(model) + " name.");
    }
};

}

namespace tachyon::json {

class JsonSerializer {
public:
    template <typename T>
    static std::string serialize(const T& value) {
        return JsonCodec<T>::toJson(value).toJsonString();
    }

    template <typename T>
    static JsonNode serializeToNode(const T& value) {
        return JsonCodec<T>::toJson(value);
    }

    template <typename T>
    static T deserialize(std::string_view json) {
        return JsonCodec<T>::fromJson(JsonNode::parse(json));
    }

    template <typename T>
    static T deserialize(const std::string& json) {
        return deserialize<T>(std::string_view(json));
    }

    template <typename T>
    static T deserialize(const char* json) {
        return deserialize<T>(std::string_view(json));
    }

    template <typename T>
    static T deserialize(const JsonNode& node) {
        return JsonCodec<T>::fromJson(node);
    }
};

}
