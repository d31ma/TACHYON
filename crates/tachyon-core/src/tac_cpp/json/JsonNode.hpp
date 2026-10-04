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
